{
  "targets": [
    {
      "target_name": "auth_session",
      "sources": ["src/auth_session.mm"],
      "defines": ["NAPI_DISABLE_CPP_EXCEPTIONS"],
      "xcode_settings": {
        "CLANG_ENABLE_OBJC_ARC": "YES",
        "MACOSX_DEPLOYMENT_TARGET": "12.0",
        "OTHER_LDFLAGS": [
          "-framework",
          "AuthenticationServices",
          "-framework",
          "AppKit"
        ]
      }
    }
  ]
}
