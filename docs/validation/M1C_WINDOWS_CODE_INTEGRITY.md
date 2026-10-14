# M1C Windows Code Integrity diagnosis

Inspection date: 2026-10-14. All displayed event times are Europe/Warsaw (UTC+02:00). This is read-only host diagnosis, with three normal Cargo test attempts. M1C source-code review is accepted; its validation gate remains blocked. No PLC implementation or security setting was changed.

## Path notation and evidence scope

`%REPO%` denotes this PLC_Student checkout. `%CARGO%` denotes `%REPO%\.tools\rustup\toolchains\1.99.0-x86_64-pc-windows-msvc\bin\cargo.exe`. `%NODE%` denotes `C:\Program Files\nodejs\node.exe`. Irrelevant account, machine-name and SID fields are omitted. Repository-relative file locations, hashes, event record IDs and activity IDs remain exact.

The collection covers CodeIntegrity/Operational events from 2026-10-14 08:20 through the controlled attempts at 09:13. Initial capture contained 11 block events for seven executable paths, with 22 activity-correlated signature events. The three controlled attempts added three block events and six signature events. Only repository executable blocks and their correlated signature/details records were retained.

Sanitized supporting JSON is stored in ignored local artifacts:

- `artifacts/m1cv/ci-events-before.json`
- `artifacts/m1cv/ci-events-after.json`
- `artifacts/m1cv/ci-smart-app-control-details.json`
- `artifacts/m1cv/binaries-before.json`

These contain selected structured event fields, not raw event XML containing the Windows user SID or computer name. They are local diagnostic artifacts, not production tooling or PLC source.

## Active policy and Windows state

| Inspection                        | Observed result                                                                                                                                          |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Win32_OperatingSystem             | Microsoft Windows 11 Pro; version 10.0.26300; x64                                                                                                        |
| Windows CurrentVersion registry   | DisplayVersion 26H2; CurrentBuild 26300; UBR 9550; EditionID Professional                                                                                |
| Smart App Control                 | Get-MpComputerStatus reports On; VerifiedAndReputablePolicyState is 1                                                                                    |
| DeviceGuard kernel/user-mode CI   | CodeIntegrityPolicyEnforcementStatus 2; UsermodeCodeIntegrityPolicyEnforcementStatus 2                                                                   |
| DeviceGuard virtualization status | VirtualizationBasedSecurityStatus 2                                                                                                                      |
| Blocking policy name              | VerifiedAndReputableDesktop, taken directly from Event 3077                                                                                              |
| Blocking policy GUID              | {0283ac0f-fff1-49ae-ada1-8a933130cad6}                                                                                                                   |
| Event PolicyID field              | 27555.1000.240208; this is separate from the PolicyGUID field                                                                                            |
| Policy hash                       | 2668895A5B233A80432D00D67251D7B7F52686A3FB13780F4B242C5A1F937A01                                                                                         |
| Policy file                       | `%SystemRoot%\System32\CodeIntegrity\CiPolicies\Active\{0283AC0F-FFF1-49AE-ADA1-8A933130CAD6}.cip`, 7,079 bytes; its SHA256 matches the event PolicyHash |
| CiTool list attempt               | `CiTool.exe -lp -json` returned OperationResult -2147024891 / 0x80070005, access denied; no elevation was requested                                      |
| Policy activation events          | Query for Event 3099 found none in the retained operational log                                                                                          |
| Session privileges                | Current token is not elevated                                                                                                                            |

The friendly name comes from actual host events and policy-file metadata, not a guess based on the GUID. Event 3077 is enforcement evidence. The policy is Smart App Control's enforcement policy; Microsoft documents the friendly-name mapping and registry value 1 as enforcement. [Microsoft Smart App Control inspection reference](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/test-your-app-with-smart-app-control)

CiTool access denial prevents a complete per-policy `IsEnforced`/`IsAuthorized` inventory. The blocking policy is confirmed active by the actual enforcement events. On-disk presence of other policies does not establish that each is enforced.

| Other policy file GUID                 | Readable name embedded in its existing CIP file                                 | Enforcement evidence in this inspection                                             |
| -------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| {0939ED82-BFD5-4D32-B58E-D31D3C49715A} | VerifiedAndReputableDesktopTestSupplemental                                     | Not independently available                                                         |
| {1283AC0F-FFF1-49AE-ADA1-8A933130CAD6} | VerifiedAndReputableDesktopEvaluation                                           | Not independently available; its presence does not make the active block audit-only |
| {1678656C-05EF-481F-BC5B-EBD8C991502D} | VerifiedAndReputableDesktopFlightSupplemental                                   | Not independently available                                                         |
| {1939ED82-BFD5-4D32-B58E-D31D3C49715A} | VerifiedAndReputableDesktopEvaluationTestSupplemental                           | Not independently available                                                         |
| {2678656C-05EF-481F-BC5B-EBD8C991502D} | VerifiedAndReputableDesktopEvaluationFlightSupplemental                         | Not independently available                                                         |
| {60FD87F8-4593-44A0-91B0-2E0DA022F248} | Microsoft Windows Endpoint Security Policy                                      | Not independently available                                                         |
| {784C4414-79F4-4C32-A6A5-F0FB42A51D0D} | Microsoft Windows Cross Certificates for Code Integrity Exceptions Audit Policy | Not independently available; name alone is not a verified audit state               |

Readable CIP metadata was inspected without converting, editing, deploying or refreshing any policy. Microsoft's CiTool reference describes the authoritative enumeration fields; that command's access denial is retained as a real limitation. [CiTool reference](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/operations/citool-commands)

Defender reports AMRunningMode `Passive Mode`, AMServiceEnabled true, AntivirusEnabled true, RealTimeProtectionEnabled false, and IsTamperProtected true. SecurityCenter2 registers Norton 360 for Gamers and Windows Defender. These were existing observations before reproduction and stayed unchanged afterward; this task did not switch antivirus modes. PartOfDomain is false, which does not establish all possible organization-management status.

## Event 3077 and correlated 3089 evidence

All collected repository blocks share:

```text
Event 3077:
  PolicyName                VerifiedAndReputableDesktop
  PolicyGUID                {0283ac0f-fff1-49ae-ada1-8a933130cad6}
  PolicyID                  27555.1000.240208
  Requested Signing Level   2
  Validated Signing Level   1
  Status                    0xc0e90002
  SI Signing Scenario       1

Correlated Event 3089:
  TotalSignatureCount       0
  SignatureType             0
  ValidatedSigningLevel     0
  VerificationError         0
  PublisherName             Unknown
  IssuerName                Unknown
  KnownRoot                 0
  Publisher/Issuer TBS hash empty
```

Match records using System/Correlation/ActivityID, not filename or approximate time alone. The 3089 Hash matches the corresponding 3077 SHA1 image hash. The event's SHA256 Flat Hash matches ordinary Get-FileHash for retained unchanged files; image/signing hashes and flat file hashes must not be conflated.

Level 2 means policy trust is required; 3077 level 1 means Windows treated the file as unsigned/no policy-approved signature. SignatureType 0 alone is ambiguous, but TotalSignatureCount 0 plus Authenticode NotSigned establishes that these files have no signature. VerificationError 0 in a zero-signature record is not evidence that the executable was allowed: the paired enforcement event records the denial. [Microsoft event-field meanings](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/operations/event-tag-explanations), [correlation and interpretation guide](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/app-control-for-business/operations/appcontrol-debugging-and-troubleshooting)

The installed Windows SDK header `Windows Kits/10/Include/10.0.22621.0/shared/ntstatus.h`, line 25393, defines 0xC0E90002 as STATUS_SYSTEM_INTEGRITY_POLICY_VIOLATION. This is a policy rejection, not a finding that the PLC program or test assertions failed.

### Every distinct historically blocked executable in the capture

Use the common signing/policy fields above for every row. Rows select the most recent original M1C block for each path, before this task's three new controlled samples. Each row has two correlated zero-signature 3089 records.

| Executable beneath %REPO%                                 | Parent  | Time on 2026-10-14 | 3077 record | 3089 records | ActivityID                             |
| --------------------------------------------------------- | ------- | ------------------ | ----------- | ------------ | -------------------------------------- |
| target/debug/deps/plc_ir-cdaf0c67f9c89440.exe             | %CARGO% | 08:36:05.0065412   | 39553       | 39552, 39554 | {b2efd5bd-535f-000d-b96f-4eb35f53dd01} |
| target/debug/deps/plc_runtime-4af21dd510c14a65.exe        | %CARGO% | 08:57:11.5755332   | 39603       | 39602, 39604 | {b2efd5bd-535f-0000-fbba-30b45f53dd01} |
| target/debug/deps/plc_types-36d56e673c74876e.exe          | %CARGO% | 08:51:45.9304529   | 39588       | 39587, 39589 | {b2efd5bd-535f-0008-352c-70b45f53dd01} |
| target/debug/deps/ton-5cfca0ee21be7838.exe                | %CARGO% | 08:45:43.2903784   | 39578       | 39577, 39579 | {b2efd5bd-535f-0003-ab94-31b55f53dd01} |
| target/debug/plc-lab-desktop.exe                          | %NODE%  | 08:53:35.1726843   | 39598       | 39597, 39599 | {b2efd5bd-535f-0008-9775-70b45f53dd01} |
| target/release/deps/semantic_runtime-2a8220562d6075d7.exe | %CARGO% | 08:44:46.6948611   | 39568       | 39567, 39569 | {b2efd5bd-535f-000f-72c6-4eb35f53dd01} |
| target/release/deps/simulation_core-6d4b72a26f83c2aa.exe  | %CARGO% | 08:52:52.7188468   | 39593       | 39592, 39594 | {b2efd5bd-535f-0000-5ab9-2fb45f53dd01} |

### Controlled reproduction events

The same common fields apply. All three Cargo invocations returned 1; the child test process never started, with Windows error 4551 (0x11C7).

| Sample                     | Time             | 3077  | 3089         | 3118  | ActivityID                             |
| -------------------------- | ---------------- | ----- | ------------ | ----- | -------------------------------------- |
| plc-types debug            | 09:12:41.6317248 | 39608 | 39607, 39609 | 39610 | {b2efd5bd-535f-0002-9ffa-80b55f53dd01} |
| simulation-core release    | 09:13:13.9115264 | 39613 | 39612, 39614 | 39615 | {b2efd5bd-535f-0000-607a-34b45f53dd01} |
| TON debug, freshly rebuilt | 09:13:17.0835881 | 39618 | 39617, 39619 | 39620 | {b2efd5bd-535f-000a-b446-a7b35f53dd01} |

Correlated Smart App Control Event 3118 adds, for each controlled sample: DefenderCalled true, DefenderCloudCallRequested true, DefenderMadeCloudCall true, DefenderClientStatusCode 0, DefenderCloudHTTPCode raw `0xc8000000`, IsUnfriendlyFile false, empty DefenderThreatName, EADefenderTrustCached false and TTLValid false. These prove the cloud evaluation path was attempted. The undocumented/raw trust and status encodings are not decoded into an invented reputation verdict or network-error explanation.

## Block classification

**BLOCK CLASS: A — unsigned/untrusted generated executable rejected by the enforced Smart App Control policy.**

The primary evidence is the exact 3077 policy/level rejection, correlated 3089 zero-signature records, Authenticode NotSigned status, and repeated rejection of a fresh normal Cargo build. Event 3118 confirms cloud authorization participation. The evidence does not identify a specific explicit deny rule, explain the cloud's per-file reputation decision, establish a parent/path restriction, or show a corrupted PE/signature hash. Therefore D (specific reputation/ISG verdict) is a participating mechanism with an undisclosed verdict, not a separately proven root cause. B/C/E/F/G are not established.

Successful unsigned executables show that an unsigned status does not universally cause rejection. It does mean there is no signing route for these files; a different policy authorization decision must account for the differing outcomes. This is an inference from the observations, not a recovered cloud score or cache allow rule. Current source and the retained original blocked release semantic-runtime binary are different hashes, so its later passing result cannot be attributed to time alone.

## Passing versus blocked binary comparison

Files were read without modification during comparison. All retained files have MZ and PE signatures, AMD64 machine 0x8664, PE32+ optional magic 0x020B, Authenticode NotSigned, no signer certificate and only the normal `:$DATA` stream. There is no Zone.Identifier/Mark-of-the-Web on these files. Profile labels come from normal target directories, not a presumption about trust.

| ID  | Repository-relative path                                  | Profile and evidence                                                                      | Bytes                   | Creation time, 2026-10-14 unless noted            |
| --- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ----------------------- | ------------------------------------------------- |
| A   | target/release/deps/plc_runtime-d5e586195a14ad57.exe      | Optimized runtime unit executable; passed in M1C                                          | 1,027,072               | 01:20:23.4335954                                  |
| B   | target/release/deps/ton-aabfa009cb691af1.exe              | Optimized TON; 14 executed passing tests in M1C                                           | 1,056,256               | 08:44:40.7498445                                  |
| C   | target/debug/deps/ton-5cfca0ee21be7838.exe                | Original blocked file missing at initial inspection; normally rebuilt and blocked again   | 1,672,192 after rebuild | Original unavailable; rebuilt at 09:13:14.9621326 |
| D   | target/release/deps/simulation_core-6d4b72a26f83c2aa.exe  | Optimized clock; blocked both historically and in controlled sample                       | 715,264                 | 01:20:23.0070703                                  |
| E   | releases/current/PLC Lab.exe                              | Retained older release; historical human-tooling report records successful release launch | 8,588,288               | 01:25:27.8711110                                  |
| F   | target/debug/plc-lab-desktop.exe                          | Newly rebuilt desktop native smoke target; blocked                                        | 12,257,792              | 2026-10-13 23:24:27.3796466                       |
| G   | target/debug/deps/plc_types-36d56e673c74876e.exe          | Debug types; blocked again in controlled sample                                           | 1,730,560               | 00:38:07.0606641                                  |
| H   | target/debug/deps/plc_runtime-4af21dd510c14a65.exe        | Workspace debug runtime; blocked                                                          | 1,630,720               | 01:15:32.9241524                                  |
| I   | target/release/deps/semantic_runtime-2a8220562d6075d7.exe | Current optimized runtime integration executable; passed after recompilation in M1C       | 1,127,936               | 01:20:24.3347540                                  |

Creation time is not build time: Cargo can overwrite a file while preserving its creation timestamp. Modification times were A 08:52:44.2156056, B 08:52:17.1337523, D 08:44:39.5705165, E 01:25:27.5445993, F 08:53:32.3001526, G 08:36:02.7682560, H 08:50:10.3702857 and I 08:52:45.3678883. Fresh C was created at 09:13:14.9621326 and modified at 09:13:15.0898061. Its measured machine/magic are 0x8664/0x020B. File C was initially absent, so no old-file Authenticode/size/creation claim is made; its normal rebuild provides a new measured sample. Its disappearance is not attributed to antivirus without evidence. No relevant Defender detection/remediation events were found in the queried log.

Full ordinary SHA256 hashes:

```text
A 475AB3E4CA9275EAFCC3138DAB14DA76B0F59669C74308D6AF2D703CD036C63E
B 24BD441DFAF6C5A238E3AE07B33415D567973D6E9BE29DCDF2614A2CF1C9D666
C 5AC3D9B8A84B8D792A087859769CD74A4C2BA15052607AFA7E1C06A3C5B46B20 (fresh rebuild)
D BEB4FFE3C80C509D9A91E8591EFB06C742290291B77CCFC61AF833BC69EE682E
E BE82CDF40C981C863DA81E7D2125E2EFE6ED8220105DEE857E5EB5E020D04A18
F 9701007637873397A27B5BC0DF6816E9272DE76BB61894E08CE553A6A715D555
G 97653D179083EBA955F0E6B3D4FA6F045F9371F47C338FC1634E0078307D232D
H A15BB01E482CDE4DD44AAD06E9CA9F67E49656891AA6816B75F6856B16029AB0
I 600878AFBDDA073B3B377079D47E5736B37D182B9695498668787C44B59C104D
```

The old C block's flat hash was A2B6E5ED59525E5BC130326F699FA567F234979D1920242DC4BCEA8FEC1EB700. The old I block's flat hash was 94A78CED5AFE6526E52D48C0E4A60082A3F733F7D944816D1E03F451BED011FC. These differ from current rebuilt bytes; no hash was silently reclassified as passing.

No block-event signing level is available for A/B/E's historical successful launches. CodeIntegrity block records are not a complete allow log; absence of a block alone does not prove execution. Their historical success comes from actual M1C test output and the human-tooling report. E is a retained release comparison artifact, not an M1C native acceptance substitute. The older successful debug desktop was overwritten by the current build, so its old bytes cannot be independently compared.

## Benign integrity and environment checks

- The regenerated C, unchanged G and D have five PE sections; the last raw section ends exactly at file length (1,672,192 / 1,730,560 / 715,264 bytes). None is zero, truncated at a declared raw section, or an architecture mismatch.
- Their hashes were read again after inspection and remained identical. The fresh C hash equals its new 3077 flat hash; G/D match their original and new block-event flat hashes.
- MSVC dumpbin `/dependents` successfully read A/C/D/E/F. The tests share the same imported runtime DLL names; both desktop builds have the expected Windows system/API-set dependencies. VCRUNTIME140.dll, ucrtbase.dll and all directly named system DLLs were found in System32. API-set imports are Windows contracts, not proof that an identically named standalone DLL must exist. No missing direct DLL was identified; this is not a full loader execution test.
- TEMP and TMP both point to the existing user's Local/AppData/Temp; documentation redacts the account portion. CARGO_TARGET_DIR is unset; the Rust wrapper does not override it. The resolved target is the normal `%REPO%\target`, inside the verified workspace.
- Sample Cargo.toml, rust.mjs, TON tests and FB source files have only `:$DATA`, without Mark-of-the-Web.
- No binary was moved to a trusted directory, renamed to evade policy, interpreted, signed, or loaded through an alternate host. No Cargo directory deletion was needed: the missing test executable caused Cargo to compile/relink the existing test normally. It still failed the same policy.

## Existing signing route audit

`Cert:\CurrentUser\My` and `Cert:\LocalMachine\My` were accessible and each contained zero CodeSigningCert entries, hence zero current code-signing certificates with private keys. The installed SDK has signtool.exe, but a signing tool alone is not a trusted certificate/workflow. Repository scripts, package configuration, desktop configuration and .github contain no configured code-signing or trusted-signing workflow.

**No already-provisioned usable signing route was found in the inspected stores and repository.** External organization/HSM/cloud arrangements not exposed here cannot be ruled out. No enrollment, purchase, certificate creation/import, trust-store modification or file signing was attempted. Any future legitimate signing route must meet the actual policy and be owner-authorized. [Microsoft signing guidance](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control)

## Exact inspection and reproduction commands

All commands used normal, non-elevated PowerShell in the repository. No registry command below writes a value.

```powershell
Get-CimInstance Win32_OperatingSystem
Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy'
Get-MpComputerStatus
Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard
Get-CimInstance Win32_ComputerSystem
Get-CimInstance -Namespace root\SecurityCenter2 -ClassName AntivirusProduct
& "$env:SystemRoot\System32\CiTool.exe" -lp -json
Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-CodeIntegrity/Operational';Id=3099} -MaxEvents 40
Get-ChildItem "$env:SystemRoot\System32\CodeIntegrity\CiPolicies\Active" -File
Get-ChildItem "$env:SystemRoot\schemas\CodeIntegrity\ExamplePolicies" -Filter '*Smart*'
Get-ChildItem Cert:\CurrentUser\My,Cert:\LocalMachine\My -CodeSigningCert
rg -n -i 'signing|signtool|trusted.?signing|codesign|certificate|azure.?sign' scripts .github apps/desktop/src-tauri/tauri.conf.json package.json docs/DEVELOPMENT.md
Get-AuthenticodeSignature -LiteralPath <existing-executable-path>
Get-FileHash -LiteralPath <existing-executable-path> -Algorithm SHA256
Get-Item -LiteralPath <existing-executable-or-source-path> -Stream *
& 'C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\dumpbin.exe' /dependents target/release/deps/plc_runtime-d5e586195a14ad57.exe target/debug/deps/ton-5cfca0ee21be7838.exe target/release/deps/simulation_core-6d4b72a26f83c2aa.exe 'releases/current/PLC Lab.exe' target/debug/plc-lab-desktop.exe
Select-String -LiteralPath 'C:\Program Files (x86)\Windows Kits\10\Include\10.0.22621.0\shared\ntstatus.h' -Pattern 'STATUS_SYSTEM_INTEGRITY_POLICY_VIOLATION' -Context 2,6
Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-Windows Defender/Operational';Id=1116,1117;StartTime=[datetime]'2026-10-14T08:20:00'}

node scripts/rust.mjs test --locked -p plc-types --lib function_blocks::tests::ton_signature_is_authoritative_typed_and_independent_of_definition_id -- --exact
node scripts/rust.mjs test --release --locked -p simulation-core --lib clock::tests::starts_at_zero_and_advances_exactly_without_wall_clock -- --exact
node scripts/rust.mjs test --locked -p plc-runtime --test ton exact_100ms_cycle_300ms_preset_uses_scan_start_and_caps_elapsed -- --exact --nocapture
```

The selected-field JSON outputs and tables above record the results. Binary PE inspection used `[IO.File]::ReadAllBytes`, DOS e_lfanew at 0x3c, COFF machine/section fields, optional-header magic and section raw ranges; it did not execute the file. CIP name inspection decoded existing bytes as UTF-16 and extracted readable metadata only, without treating that as an authoritative enforcement-state decoder.

The exact event filter/correlation procedure was:

```powershell
$taskAll = @(Get-WinEvent -FilterHashtable @{
  LogName='Microsoft-Windows-CodeIntegrity/Operational'
  Id=3077,3089
  StartTime=[datetime]'2026-10-14T08:20:00'
})
$taskParsed = @($taskAll | ForEach-Object {
  $taskXml = [xml]$_.ToXml()
  $taskFields = [ordered]@{}
  foreach ($taskData in $taskXml.Event.EventData.Data) {
    $taskFields[$taskData.Name] = ([string]$taskData.'#text').Replace(
      '\Device\HarddiskVolume3\Users\mjgom\Documents\PLC_Student', '%REPO%')
  }
  [pscustomobject]@{
    Time=$_.TimeCreated.ToString('o'); Id=$_.Id; RecordId=$_.RecordId
    ActivityId=[string]$taskXml.Event.System.Correlation.ActivityID
    Fields=$taskFields
  }
})
$taskBlocks = @($taskParsed | Where-Object {
  $_.Id -eq 3077 -and $_.Fields['File Name'] -match '^%REPO%\\.*\.exe$'
})
$taskActivities = @($taskBlocks.ActivityId | Select-Object -Unique)
$taskRelevant = @($taskParsed | Where-Object { $_.ActivityId -in $taskActivities })
```

The account-bearing redaction prefix in that command is solely the known repository path. No unrelated user paths or Windows account identifiers are retained in the artifacts. Event 3118 was queried with the same start time and selected using the already matched repository ActivityIDs.

## Disposition

SECURITY SETTINGS MODIFIED: NO

Three controlled samples establish a persistent host-policy block, including normal regeneration of the missing debug test. No provisioned signing route was found. Per the M1C-V stop rule, do not modify PLC code or security configuration to recover validation. Owner options and original-gate disposition are recorded in [M1C_VALIDATION_RECOVERY_REPORT.md](M1C_VALIDATION_RECOVERY_REPORT.md). M2 remains unauthorized.
