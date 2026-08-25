import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'connect',
      component: () => import('@/views/ConnectView.vue'),
    },
    {
      path: '/remote/:deviceId',
      name: 'remote',
      component: () => import('@/views/RemoteView.vue'),
      props: true,
    },
  ],
})

export default router
