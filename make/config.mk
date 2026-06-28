# SpeakType build configuration
#
# Set signing variables via environment, or copy make/config.example → make/local.mk
# (local.mk is gitignored and never committed).

-include make/local.mk

export NOTARY_PROFILE ?= SpeakType

ifndef APPLE_DEVELOPER_ID
  $(warning APPLE_DEVELOPER_ID not set — export it or add to make/local.mk before make build)
endif

ifndef APPLE_TEAM_ID
  $(warning APPLE_TEAM_ID not set — export it or add to make/local.mk before make build)
endif
