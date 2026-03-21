use crate::{
    app_state::AppState,
    error::ApiResult,
    modules::{
        auth::service,
        storage::{
            dto::{StorageListResponse, StorageMutationResponse},
            service::{
                self as storage_service, CreateFolderInput, StorageListQuery, UploadFileInput,
            },
        },
    },
};
use axum::{
    Json,
    extract::{Multipart, Query, State},
    http::HeaderMap,
};
use rand_core::{OsRng, RngCore};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    env,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{fs::File, io::AsyncWriteExt};

const MAX_UPLOAD_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStorageRequest {
    pub path: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequest {
    pub parent_path: Option<String>,
    pub name: String,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListStorageRequest>,
) -> ApiResult<Json<StorageListResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    let result = storage_service::list_user_storage(
        &state.pool,
        &current_user,
        StorageListQuery {
            path: query.path,
            search: query.q,
        },
    )
    .await?;

    Ok(Json(StorageListResponse {
        current_path: result.current_path,
        parent_path: result.parent_path,
        entries: result.entries,
        total_storage_limit_bytes: result.total_storage_limit_bytes,
        total_storage_used_bytes: result.total_storage_used_bytes,
        user_storage_quota_bytes: result.user_storage_quota_bytes,
        user_storage_used_bytes: result.user_storage_used_bytes,
    }))
}

pub async fn create_folder(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateFolderRequest>,
) -> ApiResult<Json<StorageMutationResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    let entry = storage_service::create_folder(
        &state.pool,
        &current_user,
        CreateFolderInput {
            parent_path: payload.parent_path,
            name: payload.name,
        },
    )
    .await?;

    Ok(Json(StorageMutationResponse {
        message: "Folder created successfully".to_owned(),
        entry,
    }))
}

pub async fn upload_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult<Json<StorageMutationResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    let mut folder_path: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut temp_file_path: Option<PathBuf> = None;
    let mut total_size_bytes: u64 = 0;
    let mut hasher = Sha256::new();

    while let Some(mut field) = multipart.next_field().await.map_err(|_| {
        crate::error::ApiError::BadRequest("Invalid multipart upload payload".to_owned())
    })? {
        match field.name() {
            Some("path") => {
                folder_path = Some(field.text().await.map_err(|_| {
                    crate::error::ApiError::BadRequest("Invalid target folder path".to_owned())
                })?);
            }
            Some("file") => {
                let detected_file_name = field
                    .file_name()
                    .map(|value| value.to_owned())
                    .ok_or_else(|| {
                        crate::error::ApiError::BadRequest("Missing uploaded file name".to_owned())
                    })?;
                let temp_path = build_temp_upload_path(current_user.user.id, &detected_file_name)?;
                let mut temp_file = File::create(&temp_path).await.map_err(|_| {
                    crate::error::ApiError::internal_with_context(
                        "Failed to open temporary upload file",
                    )
                })?;

                while let Some(chunk) = field.chunk().await.map_err(|_| {
                    crate::error::ApiError::BadRequest("Invalid file upload stream".to_owned())
                })? {
                    total_size_bytes = total_size_bytes
                        .checked_add(chunk.len() as u64)
                        .ok_or_else(|| {
                            crate::error::ApiError::BadRequest(
                                "Uploaded file is too large".to_owned(),
                            )
                        })?;

                    if total_size_bytes > MAX_UPLOAD_SIZE_BYTES {
                        let _ = std::fs::remove_file(&temp_path);
                        return Err(crate::error::ApiError::BadRequest(
                            "Uploaded file exceeds the 5 GB limit".to_owned(),
                        ));
                    }

                    hasher.update(&chunk);
                    temp_file.write_all(&chunk).await.map_err(|_| {
                        crate::error::ApiError::internal_with_context(
                            "Failed to persist uploaded file chunk",
                        )
                    })?;
                }

                temp_file.flush().await.map_err(|_| {
                    crate::error::ApiError::internal_with_context(
                        "Failed to flush temporary upload file",
                    )
                })?;

                file_name = Some(detected_file_name);
                content_type = field.content_type().map(|value| value.to_owned());
                temp_file_path = Some(temp_path);
            }
            _ => {}
        }
    }

    let file_name = file_name.ok_or_else(|| {
        crate::error::ApiError::BadRequest("Missing uploaded file name".to_owned())
    })?;
    let temp_file_path = temp_file_path
        .ok_or_else(|| crate::error::ApiError::BadRequest("Missing uploaded file".to_owned()))?;
    let file_size_bytes = i64::try_from(total_size_bytes)
        .map_err(|_| crate::error::ApiError::BadRequest("Uploaded file is too large".to_owned()))?;
    let checksum = hex::encode(hasher.finalize());

    let entry = storage_service::upload_file(
        &state.pool,
        &current_user,
        UploadFileInput {
            folder_path,
            file_name,
            content_type,
            temp_file_path: temp_file_path.clone(),
            file_size_bytes,
            checksum,
        },
    )
    .await
    .map_err(|error| {
        let _ = std::fs::remove_file(&temp_file_path);
        error
    })?;

    Ok(Json(StorageMutationResponse {
        message: "File uploaded successfully".to_owned(),
        entry,
    }))
}

fn build_temp_upload_path(
    user_id: i64,
    file_name: &str,
) -> Result<PathBuf, crate::error::ApiError> {
    let mut random = [0_u8; 8];
    OsRng.fill_bytes(&mut random);
    let random_hex = hex::encode(random);
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| {
            crate::error::ApiError::internal_with_context("Failed to generate upload timestamp")
        })?
        .as_millis();

    let extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");

    let temp_dir = env::temp_dir().join("pcloud-upload");
    std::fs::create_dir_all(&temp_dir).map_err(|_| {
        crate::error::ApiError::internal_with_context("Failed to create upload temp directory")
    })?;

    Ok(temp_dir.join(format!(
        "u{user_id}-{unix_ms}-{random_hex}.{extension}.part"
    )))
}
