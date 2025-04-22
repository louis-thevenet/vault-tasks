# Output ./demo_calendar.gif
# Output ./demo_calendar.mp4

Set Shell fish
Set Theme "Builtin Solarized Light"
Set TypingSpeed 80ms
Set FontSize 22
Set Width 1800
Set Height 1000

Hide
Type "vault-tasks -c ../.config/ -v ../test-vault calendar"
Enter
Show
Screenshot demo_calendar_view.png
Type "aaa" # so that the screenshot is actually produced
Ctrl+c

