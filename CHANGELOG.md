# 0.2.2

Bug fixes
- #11 Fix missing whitespace in Performance Data string.
  
# 0.2.1

Bug fixes
- #9 Fix a bug where negative numbers and thresholds were interpreted as switch arguments
- Update dependencies to fix known security issues

# 0.2.0

Features
- #6 PowerShell-like syntax parser for forwarded arguments, for compatibilty with vanilla Icinga/icinga-powershell-apichecks

Bug fixes
- #4 Disk and Service check were not working because arguments were forwarded as raw string
