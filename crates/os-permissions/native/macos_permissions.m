#import <AVFoundation/AVFoundation.h>
#import <CoreBluetooth/CoreBluetooth.h>
#import <CoreServices/CoreServices.h>
#import <EventKit/EventKit.h>
#import <Foundation/Foundation.h>

int os_permissions_bluetooth_authorization(void) {
    if (@available(macOS 10.15, *)) {
        return (int)CBManager.authorization;
    }
    return -1;
}

int os_permissions_microphone_authorization(void) {
    return (int)[AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio];
}

int os_permissions_microphone_request(void) {
    __block int result = -1;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(BOOL granted) {
        result = granted ? 3 : 2;
        dispatch_semaphore_signal(done);
    }];
    if (dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, 60LL * NSEC_PER_SEC)) != 0) {
        return -1;
    }
    return result;
}

int os_permissions_reminders_authorization(void) {
    return (int)[EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
}

int os_permissions_reminders_request(void) {
    __block int result = -1;
    dispatch_semaphore_t done = dispatch_semaphore_create(0);
    EKEventStore *store = [[EKEventStore alloc] init];
    void (^finish)(BOOL) = ^(BOOL granted) {
        if (granted) {
            result = 3;
        } else {
            result = (int)[EKEventStore authorizationStatusForEntityType:EKEntityTypeReminder];
            if (result == 3) {
                result = 2;
            }
        }
        dispatch_semaphore_signal(done);
    };
    if (@available(macOS 14.0, *)) {
        [store requestFullAccessToRemindersWithCompletion:^(BOOL granted, NSError *error) {
            (void)store;
            (void)error;
            finish(granted);
        }];
    } else {
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        [store requestAccessToEntityType:EKEntityTypeReminder completion:^(BOOL granted, NSError *error) {
            (void)store;
            (void)error;
            finish(granted);
        }];
#pragma clang diagnostic pop
    }
    if (dispatch_semaphore_wait(done, dispatch_time(DISPATCH_TIME_NOW, 60LL * NSEC_PER_SEC)) != 0) {
        return -1;
    }
    return result;
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
