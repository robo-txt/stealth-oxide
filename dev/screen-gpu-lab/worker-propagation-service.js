self.addEventListener('install', event => self.skipWaiting())
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()))

self.addEventListener('message', event => {
  const snapshot = {
    userAgent: navigator.userAgent,
    platform: navigator.platform,
    language: navigator.language,
    languages: [...navigator.languages],
    hardwareConcurrency: navigator.hardwareConcurrency,
    deviceMemory: navigator.deviceMemory ?? null,
    webdriver: navigator.webdriver ?? null,
    locale: Intl.DateTimeFormat().resolvedOptions().locale,
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    webgl: { available: false, reason: 'ServiceWorkerGlobalScope has no OffscreenCanvas in this test' },
  }
  event.ports[0]?.postMessage(snapshot)
})
