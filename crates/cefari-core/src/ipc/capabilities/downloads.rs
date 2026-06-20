capability! {
    name: downloads,
    order: 70,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "Downloads exist across hosts, but destination visibility and storage policy differ.",
    commands: [
        Download(DownloadCommand),
    ],
    results: [
        Download(DownloadResult),
    ],
    events: [
        Download(DownloadEvent),
    ],
}
