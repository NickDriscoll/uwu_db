@echo off

set opts=-FC -GR- -EHa- -nologo -Zi
set code=%cd%
pushd idk
cl %opts% %code%\what -Fe jgfld
popd
