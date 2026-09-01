const gpu = () => {
  try {
    const canvas = new OffscreenCanvas(16, 16)
    const gl = canvas.getContext('webgl') || canvas.getContext('webgl2')
    const debug = gl?.getExtension('WEBGL_debug_renderer_info')
    return {
      available: !!gl,
      vendor: debug ? gl.getParameter(debug.UNMASKED_VENDOR_WEBGL) : null,
      renderer: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : null,
      contextLost: gl ? gl.isContextLost() : null,
    }
  } catch (error) { return { available: false, error: String(error) } }
}

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
  webgl: gpu(),
})

self.onconnect = event => {
  const port = event.ports[0]
  port.onmessage = () => port.postMessage(snapshot())
  port.start()
}
