capability! {
    name: shell,
    order: 30,
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
