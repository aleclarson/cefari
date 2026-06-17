# Notification Behavior

Cefari owns desktop notification permission checks and delivery through the
native runtime. App UI code should use `cefari/app` notification helpers rather
than calling platform notification APIs directly.

For TypeScript usage, see [Namespace APIs](typescript/namespaces.md).

## Startup Behavior

Cefari prepares notification support during desktop startup using the configured
app identifier and display name.

Startup does not send a notification and does not request notification
permission. Permission prompts must be triggered by an explicit user-visible
flow.

## Request Rules

Notification requests require a non-empty title. Optional text fields are
trimmed and rejected when blank.

The TypeScript helper accepts simple notifications:

```ts
await cefari.notifications.send({
  title: "Build complete",
  body: "The package is ready.",
});
```

It also exposes the richer native notification contract:

```ts
await cefari.notifications.registerCategories([
  {
    id: "message",
    actions: [
      { type: "action", id: "open", title: "Open" },
      {
        type: "textInput",
        id: "reply",
        title: "Reply",
        inputButtonTitle: "Send",
        inputPlaceholder: "Message",
      },
    ],
  },
]);

await cefari.notifications.send({
  title: "Build complete",
  body: "The package is ready.",
  subtitle: "Release",
  image: { source: "appResource", path: "images/build.png" },
  icon: { source: "appData", path: "icons/build.png" },
  iconRoundCrop: true,
  threadId: "builds",
  categoryId: "message",
  userInfo: { buildId: "123" },
  xdgCategory: "transferComplete",
});
```

Media fields use Cefari-controlled app-resource or app-data references. The
frontend API does not expose arbitrary OS paths.

The desktop runtime dispatches permission, capability, category, delivery,
management, and response-event payloads through the native IPC bridge.

## Response Behavior

Subscribe to notification responses from the frontend:

```ts
const unsubscribe = cefari.notifications.onResponse((event) => {
  console.log(event.id, event.action, event.userText, event.userInfo);
});
```

Default notification clicks emit a response event and focus the main window.
Dismiss responses emit an event without focusing the main window. Action-button
and inline-reply responses emit events with the selected action id and optional
reply text.

Packaged apps register a Cefari notification activation protocol derived from
the app identifier. Windows toast activation links use that protocol and decode
to the same `notification.response` payload shape as in-process callbacks.

## Permission Behavior

Cefari checks OS notification permission before sending. If the OS reports that
notifications are denied or undetermined, Cefari treats the request as denied
instead of crashing the app.

Prompt for permission only from a user-visible action:

```ts
button.addEventListener("click", async () => {
  const permission = await cefari.notifications.requestPermission();
  console.log(permission.allowed);
});
```

## Platform Notes

- macOS requires a real app bundle identifier before native notifications can be
  delivered.
- macOS permission prompts must happen from explicit user-visible flows.
- macOS notification behavior should be verified from a signed and notarized
  `.app` bundle using the configured bundle identifier.
- macOS is the strongest target for permission state, permission prompts,
  categories, action buttons, inline replies, thread grouping, and user info.
- Windows toast setup uses the configured app identifier and the packaged
  Cefari notification activation protocol.
- Windows supports rich toast fields and response events while running; full
  cold-start activation uses the registered protocol handler.
- Linux and other XDG desktop targets depend on the notification service
  available in the user's desktop session.
- Linux/XDG supports title, body, image/icon fields, XDG categories, and
  session-scoped response/user-info behavior depending on the notification
  daemon.
