#import <Foundation/Foundation.h>
#import <UserNotifications/UserNotifications.h>

int main(int argc, const char * argv[]) {
    @autoreleasepool {
        UNUserNotificationCenter *center = [UNUserNotificationCenter currentNotificationCenter];
        UNMutableNotificationContent *content = [[UNMutableNotificationContent alloc] init];
        content.title = @"Notification Title";
        content.body = @"Notification Description";
        content.sound = [UNNotificationSound defaultSound];

        UNNotificationRequest *request = [UNNotificationRequest requestWithIdentifier:@"UniqueIdentifier" content:content trigger:nil];
        [center addNotificationRequest:request withCompletionHandler:^(NSError * _Nullable error) {
            if (error != nil) {
                NSLog(@"Error: %@", error.localizedDescription);
            }
        }];

        // Run the main event loop to keep the program running and display the notification
        NSRunLoop *runLoop = [NSRunLoop currentRunLoop];
        [runLoop run];
    }
    return 0;
}
