FROM debian:bookworm-slim AS runtime

# This argument is defined automatically by buildx when using --platform
ARG TARGETARCH

COPY target/build /tmp/target/build

RUN echo "TARGETARCH: ${TARGETARCH}" && \
    if [ "${TARGETARCH}" = "amd64" ]; then \
        export TARGET="x86_64-unknown-linux-musl" ; \
    elif [ "${TARGETARCH}" = "arm64" ]; then \
        export TARGET="aarch64-unknown-linux-musl" ; \
    elif [ "${TARGETARCH}" = "arm" ]; then \
        export TARGET="armv7-unknown-linux-musleabihf" ; \
    else \
        echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; \
        exit 1; \
    fi && \
    cp "/tmp/target/build/$TARGET/$TARGET/release/radcam-manager" "/radcam-manager" &&\
    \rm -rf "/tmp/target"

WORKDIR /

LABEL version="0.2.0-beta.11"

EXPOSE 8080/tcp

# Add docker configuration
LABEL permissions="{ \"ExposedPorts\": { \"8080/tcp\": {} }, \"HostConfig\": { \"Binds\": [ \"/var/logs/blueos/extensions/radcam-manager:/logs\", \"/usr/blueos/extensions/radcam-manager:/app\", \"/root/.config/blueos/ardupilot-manager/firmware/scripts:/scripts\" ], \"ExtraHosts\": [ \"blueos.internal:host-gateway\" ], \"PortBindings\": { \"8080/tcp\": [ { \"HostPort\": \"\" } ] } } }"
LABEL authors="[ { \"name\": \"João Antônio Cardoso\", \"email\": \"joao.maker@gmail.com\" } ]"
LABEL company="{ \"about\": \"RadCam's official management interface\", \"name\": \"Blue Robotics\", \"email\": \"support@bluerobotics.com\" }"
LABEL type="device-integration"
LABEL readme="https://raw.githubusercontent.com/bluerobotics/radcam-manager/{tag}/README.md"
LABEL links="{ \"website\": \"https://raw.githubusercontent.com/bluerobotics/radcam-manager/\", \"support\": \"https://raw.githubusercontent.com/bluerobotics/radcam-manager/\" }"
LABEL tags="[ \"rov\", \"camera\", \"cam\", \"radcam\", \"control\" ]"
LABEL requirements="[ \"core >= 1.4.3\", \"cockpit >= 1.7\" ]"

ENTRYPOINT [ \
    "./radcam-manager", \
    "--web-server", "0.0.0.0:8080", \
    "--mcm-address", "blueos.internal:6020", \
    "--mavlink", "udpout:blueos.internal:11001", \
    "--mavlink-system-id", "$MAV_SYSTEM_ID", \
    "--mavlink-component-id", "56", \
    "--log-path", "/logs", \
    "--settings-file", "/app/settings.json", \
    "--autopilot-scripts-file", "/scripts/radcam.lua", \
    "--blueos-address", "blueos.internal" \
]
