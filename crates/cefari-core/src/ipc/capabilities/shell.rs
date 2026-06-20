capability! {
    name: shell,
    order: 30,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "External URL and shell-style actions map to different host mechanisms.",
    commands: [
        OpenLogs,
        ReloadUi,
        OpenExternalUrl(OpenExternalUrlRequest),
    ],
    results: [
        ReloadUi,
        ExternalUrl(ExternalUrlResult),
    ],
    events: [
        DeepLinkOpened(DeepLinkOpenEvent),
    ],
}
