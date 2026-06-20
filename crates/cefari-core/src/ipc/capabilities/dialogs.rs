capability! {
    name: dialogs,
    order: 90,
    support: hostSpecific,
    targets: [desktop, ios, android],
    rationale: "Native dialog intent is shared, but presentation and modality differ by host.",
    commands: [
        Dialog(DialogCommand),
    ],
    results: [
        Dialog(DialogResult),
    ],
    events: [
    ],
}
