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