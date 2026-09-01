use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;

use crate::error::{Error, Result};
use crate::profiles::GpuProfile;

/// Registers the experimental WebGL string-surface patch for this page and
/// every document/iframe created from it.
pub async fn apply(page: &Page, profile: &GpuProfile) -> Result<()> {
    page.execute(AddScriptToEvaluateOnNewDocumentParams::new(source(
        profile,
    )?))
    .await
    .map_err(|source| Error::cdp("gpu surface", source))?;
    Ok(())
}

/// Builds the same source used for page documents and paused worker targets.
pub(crate) fn source(profile: &GpuProfile) -> Result<String> {
    let profile = serde_json::to_string(&serde_json::json!({
        "vendor": profile.vendor,
        "renderer": profile.renderer,
        "unmaskedVendor": profile.unmasked_vendor,
        "unmaskedRenderer": profile.unmasked_renderer,
        "version": profile.version,
        "shadingLanguageVersion": profile.shading_language_version,
    }))
    .map_err(Error::target_command_json)?;

    Ok(format!(
        r#"(() => {{
    const profile = {profile};
    const marker = Symbol.for('stealth-oxide.gpu.surface.v1');
    const state = globalThis[marker] || {{ contexts: new WeakMap() }};
    if (!globalThis[marker]) {{
        Object.defineProperty(globalThis, marker, {{ value: state, configurable: false }});
    }}

    const patchContext = context => {{
        if (!context || state.contexts.has(context)) return state.contexts.get(context) || context;

        const nativeGetParameter = context.getParameter.bind(context);
        const nativeGetExtension = context.getExtension.bind(context);
        const nativeGetSupportedExtensions = context.getSupportedExtensions.bind(context);
        const proxy = new Proxy(context, {{
            get(target, property) {{
                if (property === 'getParameter') {{
                    return parameter => {{
                        switch (parameter) {{
                            case 0x1F00: return profile.vendor;
                            case 0x1F01: return profile.renderer;
                            case 0x1F02: return profile.version;
                            case 0x8B8C: return profile.shadingLanguageVersion;
                            case 0x9245: return profile.unmaskedVendor;
                            case 0x9246: return profile.unmaskedRenderer;
                            default: return nativeGetParameter(parameter);
                        }}
                    }};
                }}
                if (property === 'getExtension') return nativeGetExtension;
                if (property === 'getSupportedExtensions') return nativeGetSupportedExtensions;
                const value = Reflect.get(target, property, target);
                return typeof value === 'function' ? value.bind(target) : value;
            }}
        }});
        state.contexts.set(context, proxy);
        return proxy;
    }};

    const patchCanvas = prototype => {{
        if (!prototype || typeof prototype.getContext !== 'function' || prototype.getContext[marker]) return;
        const descriptor = Object.getOwnPropertyDescriptor(prototype, 'getContext');
        if (!descriptor || typeof descriptor.value !== 'function') return;
        const nativeGetContext = descriptor.value;
        const patchedGetContext = function(...args) {{
            const context = nativeGetContext.apply(this, args);
            const type = String(args[0] || '').toLowerCase();
            return type === 'webgl' || type === 'experimental-webgl' || type === 'webgl2'
                ? patchContext(context)
                : context;
        }};
        Object.defineProperty(patchedGetContext, marker, {{ value: true }});
        try {{ Object.defineProperty(prototype, 'getContext', {{ ...descriptor, value: patchedGetContext }}); }} catch (_) {{}}
    }};

    patchCanvas(globalThis.HTMLCanvasElement && HTMLCanvasElement.prototype);
    patchCanvas(globalThis.OffscreenCanvas && OffscreenCanvas.prototype);
}})()"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_contains_all_experimental_string_surfaces() {
        let source = source(&GpuProfile::mesa_amd_renoir()).expect("source should serialize");
        for value in [
            "0x9245",
            "0x9246",
            "shadingLanguageVersion",
            "OffscreenCanvas",
            "webgl",
        ] {
            assert!(source.contains(value), "missing {value}");
        }
    }
}
