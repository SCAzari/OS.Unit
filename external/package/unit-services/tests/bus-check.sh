#!/bin/bash
# Live check of the three services on a private session bus (UNIT-OS-WP-0009).
# Usage: dbus-run-session -- tests/bus-check.sh <folder with the built programs>
# Starts the services with an empty profile and one test application, calls each interface as
# the Shell UI and an application would, and fails on the first wrong answer.
set -u
bin=$(realpath "$1")
work=$(mktemp -d)
trap 'kill $(jobs -p) 2>/dev/null; rm -rf "$work"' EXIT
export HOME=$work/home XDG_CONFIG_HOME=$work/home/config XDG_STATE_HOME=$work/home/state
export XDG_DATA_HOME=$work/none XDG_DATA_DIRS=$work/share
mkdir -p "$work/share/applications"
printf '[Desktop Entry]\nType=Application\nName=Sleeper\nExec=sleep 1\n' > "$work/share/applications/sleeper.desktop"

"$bin/unit-settings" & "$bin/unit-notifications" & "$bin/unit-session" &
for name in os.unit.Settings1 org.freedesktop.Notifications os.unit.Notifications1 os.unit.Session1; do
  for _ in $(seq 50); do
    gdbus introspect --session -d $name -o / >/dev/null 2>&1 && break
    sleep 0.1
  done
done

failures=0
expect() {  # expect <description> <pattern> <command...>
  local what=$1 pattern=$2; shift 2
  local out; out=$("$@" 2>&1)
  if grep -qE -- "$pattern" <<<"$out"; then
    echo "ok: $what"
  else
    echo "FAILED: $what: expected /$pattern/, got: $out"; failures=$((failures + 1))
  fi
}
call() { local dest=$1 path=$2 method=$3; shift 3; gdbus call --session -d "$dest" -o "$path" -m "$method" "$@"; }
settings() { call os.unit.Settings1 /os/unit/Settings1 os.unit.Settings1.$1 "${@:2}"; }
fdo() { call org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications.$1 "${@:2}"; }
centre() { call os.unit.Notifications1 /os/unit/Notifications1 "$@"; }
session() { call os.unit.Session1 /os/unit/Session1 os.unit.Session1.$1 "${@:2}"; }

expect "glass default" '\(<66>,\)' settings Get appearance.glass-opacity-light
expect "glass below the readability limit refused" 'Error.OutOfRange' settings Set appearance.glass-opacity-light '<59>'
expect "unknown theme refused" 'Error.NotAnOption' settings Set appearance.theme '<"neon">'
expect "theme set" '^\(\)$' settings Set appearance.theme '<"pebble">'
expect "theme saved to the profile" '^appearance.theme=pebble$' cat "$XDG_CONFIG_HOME/unit/settings"
expect "limits described" "'max': <32>" settings Describe appearance.corner-radius
expect "reset to Daylight" '^\(\)$' settings Reset appearance.
expect "theme back to Daylight" 'daylight' settings Get appearance.theme

expect "notification accepted" '\(uint32 1,\)' fdo Notify test 0 '' Hello Body '["default","Open"]' '{}' 5000
expect "server information" "'1.2'" fdo GetServerInformation
expect "Do not disturb on" '^\(\)$' centre org.freedesktop.DBus.Properties.Set os.unit.Notifications1 DoNotDisturb '<true>'
expect "Do not disturb saved" '^do-not-disturb=true$' cat "$XDG_STATE_HOME/unit/notifications"
expect "history" "'Hello', 'Body'" centre os.unit.Notifications1.History
expect "dismissed" '\(true,\)' centre os.unit.Notifications1.Dismiss 1
expect "history empty" '\[\],\)$' centre os.unit.Notifications1.History

expect "applications listed" "\('sleeper', 'Sleeper'" session ListApps
expect "unknown application refused" 'Error.UnknownApp' session Launch nope
gdbus monitor --session -d os.unit.Session1 > "$work/monitor" 2>&1 &
sleep 0.3
expect "application started" '\(uint32 [0-9]+,\)' session Launch sleeper
expect "unregistered application closed with notice" "'not-registered'" session PrepareClose sleeper
# gdbus leaves the bus right after registering, so asking it to save fails.
expect "registered" '^\(\)$' call os.unit.Session1 /os/unit/Lifecycle1 os.unit.Lifecycle1.Register sleeper
expect "save fails when the application is gone" "'failed'" session PrepareClose sleeper
sleep 1.5
expect "exit reported" "AppExited \('sleeper', uint32 [0-9]+, 0\)" cat "$work/monitor"

if [ $failures -ne 0 ]; then
  echo "$failures check(s) failed"
  exit 1
fi
echo "all bus checks passed"
