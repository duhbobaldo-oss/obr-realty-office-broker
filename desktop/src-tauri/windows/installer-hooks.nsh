!macro NSIS_HOOK_PREUNINSTALL
  ; يولّد التثبيت التالي هوية جديدة، مع إبقاء mawasib.db ومجلد files وبقية بيانات العميل.
  ExecWait '"$INSTDIR\mawasib-obr.exe" --remove-license-installation-state'
!macroend
