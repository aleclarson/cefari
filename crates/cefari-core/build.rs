use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct Capability {
    name: String,
    order: u32,
    event_order: u32,
    support: String,
    targets: Vec<String>,
    rationale: String,
    commands: Vec<String>,
    results: Vec<String>,
    events: Vec<String>,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let capability_dir = manifest_dir.join("src/ipc/capabilities");
    println!("cargo:rerun-if-changed={}", capability_dir.display());

    let mut capabilities = read_capabilities(&capability_dir);
    capabilities.sort_by_key(|capability| (capability.order, capability.name.clone()));
    validate_capabilities(&capabilities);

    let output = render_generated_glue(&capabilities);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    fs::write(out_dir.join("ipc_generated.rs"), output).expect("write generated IPC glue");
}

fn read_capabilities(capability_dir: &Path) -> Vec<Capability> {
    let mut paths = fs::read_dir(capability_dir)
        .expect("read IPC capability metadata directory")
        .map(|entry| entry.expect("read IPC capability metadata entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect::<Vec<_>>();
    paths.sort();

    paths
        .into_iter()
        .map(|path| read_capability(&path))
        .collect()
}

fn read_capability(path: &Path) -> Capability {
    println!("cargo:rerun-if-changed={}", path.display());
    let source = fs::read_to_string(path).expect("read IPC capability metadata");
    let mut name = None;
    let mut order = None;
    let mut event_order = None;
    let mut support = None;
    let mut targets = None;
    let mut rationale = None;
    let mut commands = Vec::new();
    let mut results = Vec::new();
    let mut events = Vec::new();
    let mut section = None;

    for raw_line in source.lines() {
        let line = raw_line
            .split_once("//")
            .map_or(raw_line, |(line, _comment)| line)
            .trim();
        if line.is_empty() || line == "capability! {" || line == "}" {
            continue;
        }
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(clean_scalar(value));
            continue;
        }
        if let Some(value) = line.strip_prefix("order:") {
            order = Some(
                clean_scalar(value)
                    .parse::<u32>()
                    .expect("capability order must be an integer"),
            );
            continue;
        }
        if let Some(value) = line.strip_prefix("event_order:") {
            event_order = Some(
                clean_scalar(value)
                    .parse::<u32>()
                    .expect("capability event order must be an integer"),
            );
            continue;
        }
        if let Some(value) = line.strip_prefix("support:") {
            support = Some(clean_scalar(value));
            continue;
        }
        if let Some(value) = line.strip_prefix("targets:") {
            targets = Some(clean_list(value));
            continue;
        }
        if let Some(value) = line.strip_prefix("rationale:") {
            rationale = Some(clean_scalar(value));
            continue;
        }
        if let Some(section_name) = line.strip_suffix(": [") {
            section = Some(section_name.to_owned());
            continue;
        }
        if line == "]," {
            section = None;
            continue;
        }

        let variant = line.trim_end_matches(',').trim();
        match section.as_deref() {
            Some("commands") => commands.push(variant.to_owned()),
            Some("results") => results.push(variant.to_owned()),
            Some("events") => events.push(variant.to_owned()),
            Some(section) => panic!(
                "unknown IPC metadata section {section} in {}",
                path.display()
            ),
            None => panic!(
                "unexpected IPC metadata line `{line}` in {}",
                path.display()
            ),
        }
    }

    let order = order.unwrap_or_else(|| panic!("missing capability order in {}", path.display()));

    Capability {
        name: name.unwrap_or_else(|| panic!("missing capability name in {}", path.display())),
        order,
        event_order: event_order.unwrap_or(order),
        support: support
            .unwrap_or_else(|| panic!("missing capability support in {}", path.display())),
        targets: targets
            .unwrap_or_else(|| panic!("missing capability targets in {}", path.display())),
        rationale: rationale
            .unwrap_or_else(|| panic!("missing capability rationale in {}", path.display())),
        commands,
        results,
        events,
    }
}

fn clean_scalar(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(',')
        .trim()
        .trim_matches('"')
        .to_owned()
}

fn clean_list(value: &str) -> Vec<String> {
    let value = value.trim().trim_end_matches(',').trim();
    let value = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .expect("metadata list must be bracketed");
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').to_owned())
        .collect()
}

fn validate_capabilities(capabilities: &[Capability]) {
    let mut names = BTreeSet::new();
    let mut orders = BTreeSet::new();
    let mut command_variants = BTreeSet::new();
    let mut result_variants = BTreeSet::new();
    let mut event_variants = BTreeSet::new();

    for capability in capabilities {
        assert!(
            names.insert(capability.name.clone()),
            "duplicate IPC capability name {}",
            capability.name
        );
        assert!(
            orders.insert(capability.order),
            "duplicate IPC capability order {}",
            capability.order
        );
        validate_support(capability);
        insert_variants(&mut command_variants, &capability.commands, "command");
        insert_variants(&mut result_variants, &capability.results, "result");
        insert_variants(&mut event_variants, &capability.events, "event");
    }
}

fn validate_support(capability: &Capability) {
    assert!(
        matches!(
            capability.support.as_str(),
            "portable" | "hostSpecific" | "desktopOnly" | "mobileOnly" | "deferred"
        ),
        "unknown support class {} for IPC capability {}",
        capability.support,
        capability.name
    );
    assert!(
        !capability.targets.is_empty(),
        "missing targets for IPC capability {}",
        capability.name
    );
    let mut targets = BTreeSet::new();
    for target in &capability.targets {
        assert!(
            matches!(target.as_str(), "desktop" | "ios" | "android"),
            "unknown target {} for IPC capability {}",
            target,
            capability.name
        );
        assert!(
            targets.insert(target),
            "duplicate target {} for IPC capability {}",
            target,
            capability.name
        );
    }
}

fn insert_variants(seen: &mut BTreeSet<String>, variants: &[String], kind: &str) {
    for variant in variants {
        let name = variant
            .split_once('(')
            .map_or(variant.as_str(), |(name, _payload)| name)
            .trim();
        assert!(
            seen.insert(name.to_owned()),
            "duplicate IPC {kind} variant {name}"
        );
    }
}

fn render_generated_glue(capabilities: &[Capability]) -> String {
    let commands = capabilities
        .iter()
        .flat_map(|capability| capability.commands.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let results = capabilities
        .iter()
        .flat_map(|capability| capability.results.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let mut event_capabilities = capabilities.iter().collect::<Vec<_>>();
    event_capabilities.sort_by_key(|capability| (capability.event_order, capability.name.clone()));
    let events = event_capabilities
        .into_iter()
        .flat_map(|capability| capability.events.iter().map(String::as_str))
        .collect::<Vec<_>>();

    format!(
        "{}{}{}{}{}",
        render_enum(
            "CefariIpcCommand",
            r#"#[serde(tag = "command", content = "payload", rename_all = "camelCase")]"#,
            true,
            &commands,
        ),
        render_enum(
            "CefariIpcResult",
            r#"#[serde(tag = "result", content = "payload", rename_all = "camelCase")]"#,
            false,
            &results,
        ),
        render_enum(
            "CefariIpcEvent",
            r#"#[serde(tag = "event", content = "payload", rename_all = "camelCase")]"#,
            false,
            &events,
        ),
        render_support_metadata(capabilities),
        r#"
#[must_use]
pub fn ipc_types() -> specta::Types {
    specta::Types::default()
        .register::<CefariIpcRequest>()
        .register::<CefariIpcResponse>()
        .register::<CefariIpcEvent>()
}
"#
    )
}

fn render_support_metadata(capabilities: &[Capability]) -> String {
    let mut output = String::from(
        "#[must_use]\n\
         pub const fn ipc_capability_support() -> &'static [IpcCapabilitySupport] {\n\
         \x20   &[\n",
    );
    for capability in capabilities {
        output.push_str("        IpcCapabilitySupport {\n");
        output.push_str(&format!("            name: {:?},\n", capability.name));
        output.push_str(&format!(
            "            support: PlatformSupport::{},\n",
            support_variant(&capability.support)
        ));
        output.push_str("            targets: &[\n");
        for target in &capability.targets {
            output.push_str(&format!(
                "                CefariTarget::{},\n",
                target_variant(target)
            ));
        }
        output.push_str("            ],\n");
        output.push_str(&format!(
            "            rationale: {:?},\n",
            capability.rationale
        ));
        output.push_str("        },\n");
    }
    output.push_str("    ]\n}\n\n");
    output
}

fn support_variant(value: &str) -> &'static str {
    match value {
        "portable" => "Portable",
        "hostSpecific" => "HostSpecific",
        "desktopOnly" => "DesktopOnly",
        "mobileOnly" => "MobileOnly",
        "deferred" => "Deferred",
        _ => unreachable!("support value was validated"),
    }
}

fn target_variant(value: &str) -> &'static str {
    match value {
        "desktop" => "Desktop",
        "ios" => "Ios",
        "android" => "Android",
        _ => unreachable!("target value was validated"),
    }
}

fn render_enum(name: &str, serde_attr: &str, eq: bool, variants: &[&str]) -> String {
    let equality = if eq { "Eq, " } else { "" };
    let mut output = format!(
        "// @generated by crates/cefari-core/build.rs; do not edit by hand.\n\
         #[derive(Debug, Clone, {equality}PartialEq, Serialize, Deserialize, Type)]\n\
         {serde_attr}\n\
         pub enum {name} {{\n"
    );
    for variant in variants {
        output.push_str("    ");
        output.push_str(variant);
        output.push_str(",\n");
    }
    output.push_str("}\n\n");
    output
}
