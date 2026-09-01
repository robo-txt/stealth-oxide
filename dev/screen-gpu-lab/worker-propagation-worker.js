const snapshot = () => ({
  userAgent: navigator.userAgent,
  platform: navigator.platform,
  language: navigator.language,
  languages: [...navigator.languages],
  hardwareConcurrency: navigator.hardwareConcurrency,
  deviceMemory: navigator.deviceMemory ?? null,
  webdriver: navigator.webdriver ?? null,
  locale: Intl.DateTimeFormat().resolvedOptions().locale,
  timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
  webgl: { available: false, reason: 'GPU subprobe disabled while validating worker messaging' },
})

self.onmessage = event => self.postMessage(snapshot())
