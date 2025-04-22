# Output ./demo_filter.gif
# Output ./demo_filter.mp4

Set Shell fish
Set Theme "Builtin Solarized Light"
Set TypingSpeed 80ms
Set FontSize 22
Set Width 1800
Set Height 1000

Hide
Type "vault-tasks -c ../.config/ -v ../test-vault filter"
Enter
Show

Type "inc"
Sleep 1

Screenshot demo_filter_view.png
Type "aaa" # so that the screenshot is actually produced

# Stop vault-tasks
Ctrl+c
