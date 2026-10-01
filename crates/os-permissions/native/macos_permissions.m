#import <AVFoundation/AVFoundation.h>
#import <CoreBluetooth/CoreBluetooth.h>
#import <CoreServices/CoreServices.h>
#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>
#import <Network/Network.h>

typedef void (*PermissionCompletion)(void *, int);

int os_permissions_bluetooth_authorization(void) {
    if (@available(macOS 10.15, *)) return (int)CBManager.authorization;
    return -1;
}

@interface OsPermissionsBluetoothRequest : NSObject <CBCentralManagerDelegate>
@property(nonatomic, strong) CBCentralManager *manager;
@property(nonatomic, copy) void (^finish)(int);
@end

@implementation OsPermissionsBluetoothRequest
- (void)centralManagerDidUpdateState:(CBCentralManager *)central {
    (void)central;
    int result = os_permissions_bluetooth_authorization();
    if (result == 0 || self.finish == nil) return;
    // The completion owns the request until this delegate method returns. Clear
    // the cycle only after retaining that completion, and complete exactly once.
    void (^finish)(int) = self.finish;
    self.finish = nil;
    self.manager.delegate = nil;
    self.manager = nil;
    finish(result);
}
@end

void os_permissions_bluetooth_request(PermissionCompletion done, void *context) {
    int current = os_permissions_bluetooth_authorization();
    if (current != 0) { done(context, current); return; }
    OsPermissionsBluetoothRequest *request = [OsPermissionsBluetoothRequest new];
    void *owner = (void *)CFBridgingRetain(request);
    request.finish = ^(int result) {
        __attribute__((objc_precise_lifetime)) id retained = CFBridgingRelease(owner);
        (void)retained;
        done(context, result);
    };
    dispatch_queue_t queue = dispatch_queue_create("ai.mhome.permissions.bluetooth", DISPATCH_QUEUE_SERIAL);
    // Keep construction and delegate access on one queue, including the first callback.
    dispatch_async(queue, ^{
        request.manager = [[CBCentralManager alloc] initWithDelegate:request queue:queue
            options:@{CBCentralManagerOptionShowPowerAlertKey: @NO}];
    });
}

int os_permissions_microphone_authorization(void) {
    return (int)[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
}

void os_permissions_microphone_request(PermissionCompletion done, void *context) {
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL granted) {
        done(context, granted ? 3 : 2);
    }];
}

int os_permissions_reminders_authorization(void) {
    return (int)[EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
}

void os_permissions_reminders_request(PermissionCompletion done, void *context) {
    EKEventStore *store = [EKEventStore new];
    void (^finish)(BOOL, NSError *) = ^(BOOL granted, NSError *error) {
        (void)store; // Keep the store alive until its native request completes.
        done(context, error != nil ? -1 : (granted ? 3 : os_permissions_reminders_authorization()));
    };
    if (@available(macOS 14.0, *)) {
        [store requestFullAccessToRemindersWithCompletion:finish];
    } else {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        [store requestAccessToEntityType:EKEntityTypeReminder completion:finish];
#pragma clang diagnostic pop
    }
}

// Explicit requests install one process-owned Bonjour monitor. Passive queries
// only read the most recent observation; they never start a network operation.
static nw_browser_t networkBrowser;
static uint64_t networkGeneration;

void os_permissions_network_request(void (*changed)(int)) {
    static dispatch_queue_t queue;
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        queue = dispatch_queue_create("ai.mhome.permissions.network", DISPATCH_QUEUE_SERIAL);
    });
    dispatch_async(queue, ^{
        uint64_t generation = ++networkGeneration;
        if (networkBrowser != nil) nw_browser_cancel(networkBrowser);
        nw_browse_descriptor_t descriptor = nw_browse_descriptor_create_bonjour_service("_meow-permission._tcp", "local.");
        nw_parameters_t parameters = nw_parameters_create();
        networkBrowser = nw_browser_create(descriptor, parameters);
        nw_browser_set_queue(networkBrowser, queue);
        nw_browser_set_state_changed_handler(networkBrowser, ^(nw_browser_state_t state, nw_error_t error) {
            if (generation != networkGeneration) return;
            if (state == nw_browser_state_ready) {
                changed(0);
            } else if (state == nw_browser_state_waiting || state == nw_browser_state_failed) {
                int code = error != nil && nw_error_get_error_domain(error) == nw_error_domain_dns
                    ? nw_error_get_error_code(error) : -1;
                changed(code);
            }
            if (state == nw_browser_state_failed) {
                nw_browser_cancel(networkBrowser);
                networkBrowser = nil;
            }
        });
        nw_browser_start(networkBrowser);
    });
}

int os_permissions_automation(const char *bundle_id, int ask) {
    if (bundle_id == NULL) {
        return -1;
    }
    @autoreleasepool {
        NSString *bundle = [NSString stringWithUTF8String:bundle_id];
        if (bundle == nil) {
            return -1;
        }
        NSAppleEventDescriptor *target = [NSAppleEventDescriptor descriptorWithBundleIdentifier:bundle];
        if (target == nil || target.aeDesc == NULL) {
            return -1;
        }
        OSStatus status = AEDeterminePermissionToAutomateTarget(
            target.aeDesc,
            typeWildCard,
            typeWildCard,
            ask ? true : false
        );
        return (int)status;
    }
}
