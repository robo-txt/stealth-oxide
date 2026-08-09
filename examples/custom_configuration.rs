use stealth_oxide::{Patch, StealthConfig};

fn main() {
    // Begin with no modifications, then opt in only to the behavior your
    // application needs. Native patches leave Chromium's value untouched.
    let config = StealthConfig::none()
        .enable(Patch::Identity)
        .enable(Patch::Locale)
        .timezone("America/New_York")
        .use_native(Patch::Screen);

    let plan = config.plan();
    for (patch, state) in plan.operations() {
        println!("{patch:?}: {state:?}");
    }

    if !plan.issues().is_empty() {
        eprintln!("configuration issues: {:#?}", plan.issues());
        std::process::exit(1);
    }
}
