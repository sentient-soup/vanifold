# bridges/

Sidecar adapters that translate non-MQTT devices onto the MQTT spine, publishing
HA-discovery-format announcements like any other device. Typically small Python
daemons wrapping existing libraries (aiobmsble, victron-ble, mopeka-iot-ble).

Planned first bridge: BLE BMS telemetry via `aiobmsble` (Portable Sun battery).
