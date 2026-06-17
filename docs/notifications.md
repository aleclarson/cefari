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

The full protocol defines permission, capability, category, delivery,
management, and response-event payloads. The current desktop IPC dispatcher
still returns `unsupported` until notification dispatch is wired end to end.

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
- macOS is the strongest target for permission state, permission prompts,
  categories, action buttons, inline replies, thread grouping, and user info.
- Windows toast setup uses the configured app identifier.
- Windows supports rich toast fields and response events while running; full
  cold-start activation requires package/protocol activation wiring.
- Linux and other XDG desktop targets depend on the notification service
  available in the user's desktop session.
- Linux/XDG supports title, body, image/icon fields, XDG categories, and
  session-scoped response/user-info behavior depending on the notification
  daemon.
