# syntax=docker/dockerfile:1.7
FROM scratch
COPY --chmod=0555 deployments/external-input/vusb/profile/ /profile/
COPY --chmod=0444 deployments/external-input/vusb/profile/virtual-usb-profile.json /profile/virtual-usb-profile.json
COPY --chmod=0444 LICENSE /profile/LICENSE
COPY --chmod=0444 deployments/external-input/vusb/profile/NOTICE /profile/NOTICE
