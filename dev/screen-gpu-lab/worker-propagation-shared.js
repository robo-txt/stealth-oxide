const snapshot = () => {
  return {
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
  }
}

self.onconnect = event => {
  const port = event.ports[0]
  port.onmessage = () => port.postMessage(snapshot())
  port.start()
}
