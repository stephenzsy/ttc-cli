#!pwsh

$Env:PROTOC=$(Get-ChildItem -Path .\protoc-34.1-win64\bin\protoc.exe).FullName
cargo build