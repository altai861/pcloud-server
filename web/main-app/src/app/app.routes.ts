import { inject } from '@angular/core';
import { CanActivateFn, Router, Routes } from '@angular/router';
import { catchError, map, of } from 'rxjs';

import { AdminPageComponent } from './components/admin-page/admin-page.component';
import { HomePageComponent } from './components/home-page/home-page.component';
import { LoginPageComponent } from './components/login-page/login-page.component';
import { SharedPageComponent } from './components/shared-page/shared-page.component';
import { StarredPageComponent } from './components/starred-page/starred-page.component';
import { StorageHomeComponent } from './components/storage-home/storage-home.component';
import { TrashPageComponent } from './components/trash-page/trash-page.component';
import { WorkspaceProfileComponent } from './components/workspace-profile/workspace-profile.component';
import { WorkspaceShellComponent } from './components/workspace-shell/workspace-shell.component';
import { AuthApiService } from './services/auth-api.service';
import { ClientSessionService } from './services/client-session.service';

const loginRouteGuard: CanActivateFn = () => {
  const router = inject(Router);
  const authApi = inject(AuthApiService);
  const session = inject(ClientSessionService);
  const accessToken = session.readAccessToken();

  if (!accessToken) {
    return true;
  }

  return authApi.me('', accessToken).pipe(
    map(() => router.createUrlTree(['/app/storage'])),
    catchError(() => {
      session.clearSession();
      return of(true);
    })
  );
};

const workspaceGuard: CanActivateFn = () => {
  const router = inject(Router);
  const authApi = inject(AuthApiService);
  const session = inject(ClientSessionService);
  const accessToken = session.readAccessToken();

  if (!accessToken) {
    return router.createUrlTree(['/login']);
  }

  return authApi.me('', accessToken).pipe(
    map(() => true),
    catchError(() => {
      session.clearSession();
      return of(router.createUrlTree(['/login']));
    })
  );
};

export const routes: Routes = [
  {
    path: '',
    pathMatch: 'full',
    redirectTo: 'login'
  },
  {
    path: 'login',
    component: LoginPageComponent,
    canActivate: [loginRouteGuard]
  },
  {
    path: 'app',
    component: WorkspaceShellComponent,
    canActivate: [workspaceGuard],
    children: [
      {
        path: '',
        pathMatch: 'full',
        redirectTo: 'storage'
      },
      {
        path: 'home',
        component: HomePageComponent
      },
      {
        path: 'storage',
        component: StorageHomeComponent
      },
      {
        path: 'profile',
        component: WorkspaceProfileComponent
      },
      {
        path: 'starred',
        component: StarredPageComponent
      },
      {
        path: 'shared',
        component: SharedPageComponent
      },
      {
        path: 'admin',
        component: AdminPageComponent
      },
      {
        path: 'trash',
        component: TrashPageComponent
      }
    ]
  },
  {
    path: '**',
    redirectTo: 'login'
  }
];
