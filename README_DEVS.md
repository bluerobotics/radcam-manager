# RadCam Manager

It provides a Vue3 application to be loaded either as a BlueOS extension, and as an iframe for Cockpit.

The project has a backend with basically three services:

1. A [Mavlink Camera Manager](http://github.com/mavlink/mavlink-camera-manager) client, responsible to authenticate and setup RadCam streams.
2. A web server, responsible to serve the frontend as a static websitem as well as providing a REST API for it.
3. A RadCam protocol proxy, translating requests from the frontend to the camera itself.

## How to run

There are a few options:

### As a BlueOS extension:

1. Find the RadCam Manager on the Extensions Store, and install it.
2. A "RadCam" should appear on the left menu after a few seconds.
3. Click the "RadCam" menu item to access the application's interface.
4. Open Cockpit, there should be an available configured iframe

### As a BlueOS extension, manually:

- Extension Identifier: `bluerobotics.radcam_manager`

- Extension Name: `RadCam Manager`

- Docker image: `joaoantoniocardoso/radcam-manager`

- Docker tag: `latest`

- Original Settings:

```
{
  "ExposedPorts": {
    "8080/tcp": {}
  },
  "HostConfig": {
    "Binds": [
      "/var/logs/blueos/extensions/radcam-manager:/logs",
      "/usr/blueos/extensions/radcam-manager:/app",
      "/root/.config/blueos/ardupilot-manager/firmware/scripts:/scripts"
    ],
    "ExtraHosts": [
      "blueos.internal:host-gateway"
    ],
    "PortBindings": {
      "8080/tcp": [
        {
          "HostPort": ""
        }
      ]
    }
  },
  "entrypoint": [
    "./radcam-manager",
    "--verbose",
    "--web-server", "0.0.0.0:8080",
    "--mcm-address", "blueos.internal:6020",
    "--mavlink", "udpout:blueos.internal:11001",
    "--mavlink-system-id", "$MAV_SYSTEM_ID",
    "--mavlink-component-id", "56",
    "--log-path", "/logs",
    "--settings-file", "/app/settings.json",
    "--autopilot-scripts-file", "/scripts/radcam.lua",
    "--blueos-address", "blueos.internal"
  ]
}
```

### As a standalone application, via Docker container:

```bash
docker build . -t radcam-manager --progress=plain
docker run --rm -it --net=host radcam-manager:latest --help
```

###  As a standalone application, locally:
```bash
./build.sh
./backend/target/radcam-manager --help
```

**NOTE**: this software requires:
- [Mavlink Camera Manager](http://github.com/mavlink/mavlink-camera-manager) instance running somewhere. It can configurable using the `--mcm-address <MCM_ADDRESS>` command line argument.
- Some MAVLink-compatible autopilot firmware running with version `>=4.7.0`
- Some MAVLink router endpoint, configurable using the `--mavlink` command line argument.

