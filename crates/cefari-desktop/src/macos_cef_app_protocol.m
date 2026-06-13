#import <Cocoa/Cocoa.h>

static BOOL CefariHandlingSendEvent = NO;

void cefari_macos_cef_app_protocol_link_anchor(void) {}

// CEF asks NSApp for Chromium's app protocol while handling context menus.
@implementation NSApplication (CefariCefAppProtocol)

- (BOOL)isHandlingSendEvent {
  return CefariHandlingSendEvent;
}

- (void)setHandlingSendEvent:(BOOL)handlingSendEvent {
  CefariHandlingSendEvent = handlingSendEvent;
}

@end
