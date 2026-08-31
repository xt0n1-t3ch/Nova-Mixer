# Understand applications and audio sessions

NovaMixer stores applications and controls their live Windows audio sessions. This page explains identity, aggregate volume policy, discovery, and Component Object Model (COM) ownership.

## Applications persist after their sessions close

An `Application` is the durable object shown in the Applications view. Its `app_key` comes from the Application User Model ID, canonical executable path, or executable name, in that order.

An `AudioSession` is a live child identified by `live_id`. One application can own several sessions. NovaMixer groups those sessions under one application and keeps the application after its last session expires.

## Application controls set policy

Setting application volume writes the same scalar to every controllable child. NovaMixer stores that scalar and applies it to sessions created later. Mute uses the same behavior.

If adopted sessions disagree before NovaMixer applies policy, `mixed` is `true`. The first application volume or mute action synchronizes the children and clears `mixed`. NovaMixer never averages child values.

Application and session peaks use the highest child peak. They never sum peaks from independent streams.

## The worker discovers live sessions

NovaMixer follows the default render endpoint for the `eConsole` role. The `windows-audio` thread owns `IMMDeviceEnumerator`, `IAudioSessionManager2`, endpoint controls, meters, and session controls.

Startup registers `IAudioSessionNotification`, obtains the enumerator, and calls `GetCount()` before adopting existing sessions. Windows requires that initial call before it delivers session creation notifications. A ten second reconciliation repairs missed callbacks.

Core Audio can call `OnSessionCreated` on another COM thread. The callback places the interface in the Global Interface Table and sends its cookie to the worker. The worker resolves and revokes the cookie before it applies saved application policy and emits `application-added`.

## Session lifecycle preserves application state

Inactive sessions remain listed and controllable. Expired and disconnected sessions leave the live registry. Their parent application remains with `running: false`.

A rejected control call marks that session as uncontrollable. NovaMixer keeps the application and other child sessions available. A default endpoint change rebuilds endpoint registrations and emits one replacement `endpoint-changed` snapshot.

## Metering uses one batch

The worker samples the endpoint and session meters in one tick. It emits one `peaks` event with application and child peaks. The active interval is 50 ms, and the idle interval is 250 ms.
