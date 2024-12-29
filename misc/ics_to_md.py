from ics import Calendar
from datetime import datetime, timezone
import sys

file = open(sys.argv[1], 'r')
c = Calendar(file.read())

file.close()

now = datetime.now(timezone.utc)

for e in c.events:
    if e.begin.datetime > now: # Only add future events
        event_str = "- [ ]"
        event_str += " " + e.name
        event_str += " " + e.begin.datetime.strftime("%d/%m/%Y")
        print(event_str)
