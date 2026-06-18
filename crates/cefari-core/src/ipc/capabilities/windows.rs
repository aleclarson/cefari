capability! {
    name: windows,
    order: 20,
    commands: [
        WindowCurrent,
        WindowList,
        WindowCreate(WindowCreateRequest),
        WindowShow(WindowTargetRequest),
        WindowFocus(WindowTargetRequest),
        WindowClose(WindowTargetRequest),
        WindowSetTitle(WindowSetTitleRequest),
    ],
    results: [
        Window(WindowState),
        WindowList(WindowListResult),
    ],
    events: [
        WindowCreated(WindowStateEvent),
        WindowShown(WindowStateEvent),
        WindowFocused(WindowStateEvent),
        WindowBlurred(WindowStateEvent),
        WindowCloseRequested(WindowStateEvent),
        WindowClosed(WindowIdEvent),
        WindowMoved(WindowStateEvent),
        WindowResized(WindowStateEvent),
        WindowTitleChanged(WindowStateEvent),
    ],
}
