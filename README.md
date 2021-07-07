# HUAWEI Metrics Exporter

A single binary which scrapes the HUAWEI router web interface for all device information. Can be used as metrics for connection strenth and speed as well as data usage. It includes information which is hidden on the web interface.

It currently outputs JSON which can be piped into a file and then uploaded to a service like homeassistant or archived and analyzed manually. I am planning to implement an OpenMetrics output format at some point for use with prometheus.

```json
{
  "rsrp": {
    "label_id": "deviceinformation.rsrp",
    "label": "RSRP",
    "value_id": "di-rsrp",
    "value": "-109dBm",
    "hidden": false
  },
  "rsrq": {
    "label_id": "deviceinformation.rsrq",
    "label": "RSRQ",
    "value_id": "di-rsrq",
    "value": "-10.0dB",
    "hidden": false
  },
  "sinr": {
    "label_id": "deviceinformation.sinr",
    "label": "SINR",
    "value_id": "di-sinr",
    "value": "2dB",
    "hidden": false
  },
  "txPower": {
    "label_id": "deviceinformation.txPower",
    "label": "Wireless transmit power",
    "value_id": "di-txpower",
    "value": "PPusch:13dBm PPucch:4dBm PSrs:19dBm PPrach:17dBm",
    "hidden": false
  },
  "totalupload": {
    "label_id": "deviceinformation.totalupload",
    "label": "",
    "value_id": "deviceinformation_totalupload",
    "value": "182944.35MB",
    "hidden": true
  },
  "totaldownload": {
    "label_id": "deviceinformation.totaldownload",
    "label": "",
    "value_id": "deviceinformation_totaldownload",
    "value": "3764530.85MB",
    "hidden": true
  },
  "currentdownloadrate": {
    "label_id": "deviceinformation.currentdownloadrate",
    "label": "",
    "value_id": "deviceinformation_currentdownloadrate",
    "value": "14.93Mbps",
    "hidden": true
  },
  // ...
}
```

## Environemnt Variables

This exporter uses the following environemt variables:

- `CHROME_BINARY`: Absolute path to chrome(ium) binary to use.
- `CHROMEDRIVER_PORT`: Port at which [chromedriver][chromedriver] is running locally.
- `HUAWEI_ROUTER_HOST`: IP or hostname at which HUAWEI router web interface can be found.
- `HUAWEI_ROUTER_PASS`: Password for login on HUAWEI router web interface.

## Dotfile

At `.env` in PWD.

[chromedriver]: https://chromedriver.chromium.org/

## Example usage

### Uploading to HASS webhook trigger

```sh
#!/bin/bash

ENDPOINT=some_enpoint_id
TMPFILE=".huawei.json"
CHROMEDRIVER_PORT=9515 HUAWEI_ROUTER_PASS=Lamppenkey1 huawei-metrics > $TMPFILE
data=$(cat $TMPFILE)
curl -X POST -H "Content-Type: application/json" --data "$data\n" https://hass.local/api/webhook/$ENDPOINT
```

```yaml
template:
  - trigger:
    - platform: webhook
      webhook_id: "some_endpoint_id"
    sensor:
      - name: '{{ trigger.json.sinr.label }}'
        state: '{{ trigger.json.sinr.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.sinr.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
      - name: '{{ trigger.json.rsrq.label }}'
        state: '{{ trigger.json.rsrq.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.rsrq.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
      - name: '{{ trigger.json.rssi.label }}'
        state: '{{ trigger.json.rssi.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.rssi.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
      - name: '{{ trigger.json.serialNumber.label }}'
        state: '{{ trigger.json.serialNumber.value }}'
      - name: '{{ trigger.json.INI.label }}'
        state: '{{ trigger.json.INI.value }}'
      - name: '{{ trigger.json.totaldownload.label }}'
        state: '{{ trigger.json.totaldownload.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.totaldownload.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
      - name: '{{ trigger.json.totalupload.label }}'
        state: '{{ trigger.json.totalupload.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.totalupload.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
      - name: '{{ trigger.json.currentdownloadrate.label }}'
        state: '{{ trigger.json.currentdownloadrate.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.currentdownloadrate.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
      - name: '{{ trigger.json.currentuploadrate.label }}'
        state: '{{ trigger.json.currentuploadrate.value | regex_replace(find="[a-z]", replace="", ignorecase=True) | float }}'
        unit_of_measurement: '{{ trigger.json.currentuploadrate.value | regex_replace(find="\d+\.\d+", replace="", ignorecase=True) }}'
```