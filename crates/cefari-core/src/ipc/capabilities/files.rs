capability! {
    name: files,
    order: 100,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "File access is shared at the API level but sandboxed and permission-mediated by host.",
    commands: [
        Files(FilesCommand),
    ],
    results: [
        File(FileResult),
    ],
    events: [
    ],
}
