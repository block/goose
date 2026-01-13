#include <node_api.h>
#include <dispatch/dispatch.h>
#include <string>

#import <AppKit/AppKit.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <Foundation/Foundation.h>

@interface GooseAuthPresentationContextProvider : NSObject <ASWebAuthenticationPresentationContextProviding>
@end

@implementation GooseAuthPresentationContextProvider
- (ASPresentationAnchor)presentationAnchorForWebAuthenticationSession:
    (ASWebAuthenticationSession *)session {
  return NSApp.keyWindow ?: NSApp.mainWindow;
}
@end

typedef struct AuthRequest {
  napi_env env;
  napi_deferred deferred;
  napi_threadsafe_function tsfn;
} AuthRequest;

typedef struct AuthResult {
  bool success;
  std::string message;
} AuthResult;

static NSMutableSet<ASWebAuthenticationSession *> *gActiveSessions = nil;
static GooseAuthPresentationContextProvider *gPresentationContextProvider = nil;

static void CleanupModule(void * /*data*/) {
  [gActiveSessions removeAllObjects];
  gActiveSessions = nil;
  gPresentationContextProvider = nil;
}

static void RejectPromise(AuthRequest *request, const char *message) {
  napi_value msg;
  napi_status msg_status =
      napi_create_string_utf8(request->env, message, NAPI_AUTO_LENGTH, &msg);
  napi_value error;
  napi_value rejection;
  if (msg_status == napi_ok &&
      napi_create_error(request->env, nullptr, msg, &error) == napi_ok) {
    rejection = error;
  } else if (msg_status == napi_ok) {
    rejection = msg;
  } else {
    napi_get_undefined(request->env, &rejection);
  }
  napi_reject_deferred(request->env, request->deferred, rejection);
  if (request->tsfn) {
    napi_release_threadsafe_function(request->tsfn, napi_tsfn_release);
  }
  delete request;
}

static void CallJs(napi_env env, napi_value _js_cb, void *context, void *data) {
  AuthRequest *request = static_cast<AuthRequest *>(context);
  AuthResult *result = static_cast<AuthResult *>(data);

  if (!request || !result) {
    delete result;
    delete request;
    return;
  }

  if (env == nullptr) {
    napi_release_threadsafe_function(request->tsfn, napi_tsfn_release);
    delete result;
    delete request;
    return;
  }

  napi_handle_scope scope;
  if (napi_open_handle_scope(env, &scope) != napi_ok) {
    napi_release_threadsafe_function(request->tsfn, napi_tsfn_release);
    delete result;
    delete request;
    return;
  }

  napi_value msg;
  napi_status msg_status =
      napi_create_string_utf8(env, result->message.c_str(), NAPI_AUTO_LENGTH, &msg);
  if (result->success) {
    napi_value resolution;
    if (msg_status == napi_ok) {
      resolution = msg;
    } else {
      napi_get_undefined(env, &resolution);
    }
    napi_resolve_deferred(env, request->deferred, resolution);
  } else {
    napi_value error;
    napi_value rejection;
    if (msg_status == napi_ok && napi_create_error(env, nullptr, msg, &error) == napi_ok) {
      rejection = error;
    } else if (msg_status == napi_ok) {
      rejection = msg;
    } else {
      napi_get_undefined(env, &rejection);
    }
    napi_reject_deferred(env, request->deferred, rejection);
  }

  napi_close_handle_scope(env, scope);
  napi_release_threadsafe_function(request->tsfn, napi_tsfn_release);
  delete result;
  delete request;
}

static void SendResult(AuthRequest *request, bool success, const char *message) {
  if (!request || !request->tsfn) {
    delete request;
    return;
  }

  AuthResult *result = new AuthResult{success, message ? message : ""};
  napi_status status =
      napi_call_threadsafe_function(request->tsfn, result, napi_tsfn_blocking);
  if (status != napi_ok) {
    delete result;
    napi_release_threadsafe_function(request->tsfn, napi_tsfn_release);
    delete request;
  }
}

static bool GetString(napi_env env, napi_value value, std::string *out) {
  size_t length = 0;
  if (napi_get_value_string_utf8(env, value, nullptr, 0, &length) != napi_ok) {
    return false;
  }

  std::string result;
  result.resize(length + 1);
  if (napi_get_value_string_utf8(env, value, result.data(), result.size(), &length) != napi_ok) {
    return false;
  }

  result.resize(length);
  *out = result;
  return true;
}

static napi_value StartAuthSession(napi_env env, napi_callback_info info) {
  napi_value promise;
  napi_deferred deferred;
  napi_create_promise(env, &deferred, &promise);

  size_t argc = 2;
  napi_value args[2];
  napi_get_cb_info(env, info, &argc, args, nullptr, nullptr);

  if (argc < 2) {
    RejectPromise(new AuthRequest{env, deferred, nullptr}, "Missing arguments.");
    return promise;
  }

  std::string authUrl;
  std::string callbackScheme;
  if (!GetString(env, args[0], &authUrl) || !GetString(env, args[1], &callbackScheme)) {
    RejectPromise(new AuthRequest{env, deferred, nullptr}, "Invalid arguments.");
    return promise;
  }

  AuthRequest *request = new AuthRequest{env, deferred, nullptr};
  napi_value resource_name;
  napi_create_string_utf8(env, "authSession", NAPI_AUTO_LENGTH, &resource_name);
  if (napi_create_threadsafe_function(env,
                                      nullptr,
                                      nullptr,
                                      resource_name,
                                      0,
                                      1,
                                      nullptr,
                                      nullptr,
                                      request,
                                      CallJs,
                                      &request->tsfn) != napi_ok) {
    RejectPromise(request, "Failed to initialize auth session.");
    return promise;
  }

  dispatch_async(dispatch_get_main_queue(), ^{
    @autoreleasepool {
      NSString *authUrlString = [NSString stringWithUTF8String:authUrl.c_str()];
      NSString *schemeString = [NSString stringWithUTF8String:callbackScheme.c_str()];
      NSURL *url = authUrlString ? [NSURL URLWithString:authUrlString] : nil;

      if (!url || !schemeString) {
        SendResult(request, false, "Invalid authentication URL.");
        return;
      }

      if (!gPresentationContextProvider) {
        gPresentationContextProvider = [GooseAuthPresentationContextProvider new];
      }
      if (!gActiveSessions) {
        gActiveSessions = [NSMutableSet set];
      }

      __block ASWebAuthenticationSession *session = nil;
      session = [[ASWebAuthenticationSession alloc] initWithURL:url
                                            callbackURLScheme:schemeString
                                            completionHandler:^(NSURL * _Nullable callbackURL,
                                                                NSError * _Nullable error) {
        if (callbackURL) {
          NSString *callbackString = callbackURL.absoluteString ?: @"";
          SendResult(request, true, callbackString.UTF8String);
        } else {
          NSString *errorMessage = error.localizedDescription ?: @"Authentication canceled.";
          SendResult(request, false, errorMessage.UTF8String);
        }
        if (session) {
          [gActiveSessions removeObject:session];
        }
      }];

      if (!session) {
        SendResult(request, false, "Failed to initialize authentication session.");
        return;
      }

      session.presentationContextProvider = gPresentationContextProvider;
      session.prefersEphemeralWebBrowserSession = NO;

      [gActiveSessions addObject:session];
      if (![session start]) {
        [gActiveSessions removeObject:session];
        SendResult(request, false, "Unable to start authentication session.");
      }
    }
  });

  return promise;
}

NAPI_MODULE_INIT() {
  napi_value fn;
  napi_create_function(env, "startAuthSession", NAPI_AUTO_LENGTH, StartAuthSession, nullptr, &fn);
  napi_set_named_property(env, exports, "startAuthSession", fn);
  napi_add_env_cleanup_hook(env, CleanupModule, nullptr);
  return exports;
}
