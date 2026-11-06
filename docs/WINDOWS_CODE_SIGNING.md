# Windows code signing and Smart App Control

Status on **2026-11-03**: the existing root `PLC Lab.exe` is **UNSIGNED**. Signing
automation is implemented separately from publisher onboarding. No usable local signing
credential was found. Initial read-only GitHub inspection found no secrets, variables
or environments. The protected release environment has since been prepared with
`PLC_LAB_SIGNING_READY=false`; publisher configuration and credentials remain absent.
The root EXE has not been signed by this task.
**SMART APP CONTROL DEVICE ACCEPTANCE: NOT VERIFIED.**

The user's account type is confirmed as **individual developer in Poland**. The chosen
provider is SSL.com IV Code Signing with eSigner, pending external onboarding.

## Initial executable and release audit

The read-only audit used `Get-AuthenticodeSignature`, PE headers, version information and
SHA-256. Its machine-readable record is
[`windows-code-signing-audit.json`](windows-code-signing-audit.json).

| Property                      | Observed value                                                            |
| ----------------------------- | ------------------------------------------------------------------------- |
| File                          | `C:\Users\mjgom\Documents\PLC_Student\PLC Lab.exe`                        |
| Binary                        | PE32+ / PE64, AMD64 (`0x8664`), 16,805,376 bytes                          |
| Authenticode                  | `NotSigned` — **UNSIGNED**                                                |
| Publisher / chain / timestamp | Absent                                                                    |
| File and product version      | `0.2.0-dev`                                                               |
| SHA-256                       | `9ffb15c79e39a8757d6333e09d58340262aa591bd11cf4d70a76037d9e0b6751`        |
| Existing staging              | Matches `target/release/plc-lab-desktop.exe` and `resources/product.json` |

Matching the local build and staging manifest proves byte equality. It does not prove
that the binary came from a reviewed, validated source commit. The supported production
route rebuilds validated source, signs that output and installs the verified result.
Do not send an unknown existing EXE to a production signing account.

Follow-up on the same date: Windows reports **SmartAppControlState=On** and Defender
**AMRunningMode=Passive Mode**. Debug native smoke passed 25 steps. Launching the current
unsigned root release failed with `spawn UNKNOWN`; fresh Code Integrity events **3077**
and **3033** name the exact root `PLC Lab.exe` (policy
`{0283ac0f-fff1-49ae-ada1-8a933130cad6}`). Its initial SHA-256 is unchanged. This is actual
evidence of the unsigned release block, not a failed signed-release acceptance test.
No security setting was altered. **Signed Smart App Control acceptance remains NOT
VERIFIED** until an issued trusted certificate signs the release.

Before this change, release staging copied the unsigned Cargo executable into the root,
then recorded its SHA-256. Canonical Windows CI validated that distribution, but had no
publisher-signing step. [ADR-0039](adr/0039-windows-release-signing.md) adds the release
trust gate while preserving [ADR-0030](adr/0030-root-single-executable.md).

## Provider decision and eligibility

The user confirmed **individual developer in Poland**. The selected route is
**SSL.com Individual Validated (IV) Code Signing + eSigner for Code**, with the managed
private key retained in the provider's cloud HSM. Account onboarding is incomplete:
no certificate, publisher identity or signing credential has been issued/configured here.

[SSL.com's IV product](https://www.ssl.com/products/software-integrity/code-signing/iv/)
supports individuals without business registration and cloud signing.
Its [accepted country list](https://www.ssl.com/country-codes/) explicitly includes
**Poland (PL)**. Its [identity validation guide](https://www.ssl.com/guide/identity-validation-for-ssl-com-certificates-a-complete-guide/)
covers IV Code Signing and accepts EU biometric national identity cards/passports.
These are evidence of an application route; approval still depends on validation of
this applicant's real identity, address and documents. Do not assume issuance is guaranteed.

| Route                                | Assessment for this user                                                                                                                                                                                                                                                                                                              |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SSL.com IV + eSigner                 | Chosen public publisher-signing route, pending purchase and identity approval; preserves standalone EXE and supports cloud CI.                                                                                                                                                                                                        |
| Azure Artifact Signing Public Trust  | Unavailable to an individual located in Poland. Organizations in Poland are eligible through the EU route; individuals remain limited to the US/Canada.                                                                                                                                                                               |
| Certum Standard Code Signing         | Legitimate individual/company alternative; its [official product family](https://www.certum.eu/en/code-signing-certificates/) and [Polish activation guide](https://files.certum.eu/documents/manual_pl/Code-Signing-Standard-aktywacja-i-instalacja.pdf) support individuals. Its cloud service has not been integrated/tested here. |
| Commercial OV/EV / certificate store | Supported verification/local signing adapter after a CA accepts the real publisher and provisions a compatible HSM/token/cloud provider. EV is unnecessary solely for SmartScreen.                                                                                                                                                    |
| Store/MSIX                           | Alternative only; not adopted because the requested delivery remains the standalone root EXE.                                                                                                                                                                                                                                         |
| SignPath Foundation                  | Microsoft lists a free qualifying-open-source program; separate eligibility/acceptance required, not configured here.                                                                                                                                                                                                                 |

The [Microsoft signing-options comparison](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
describes these legitimate distribution choices. The
[Artifact Signing quickstart](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart)
is authoritative for its identity geography. A Poland Central resource endpoint
(`https://plc.codesigning.azure.net`) does not make a Polish individual eligible.
Do not create an organization identity with invented details.

SSL.com currently lists the IV certificate at **US$129/year**. The separate
[eSigner IV/OV pricing guide](https://www.ssl.com/guide/esigner-pricing-for-code-signing/)
lists Tier 1 cloud signing at **US$20/month for 20 signatures**, or **US$180/year for
240 signatures**. Thus certificate plus annual Tier 1 cloud signing is approximately
**US$309/year**, before tax, currency conversion and possible extra signing credits.
Each PLC Lab release signs at least the EXE and catalog: **two signatures per release**.
These published prices are estimates, not a purchase or binding quote. Confirm current
Polish eligibility, exact certificate/cloud plan, allowances and total cost before payment.

## What Windows verifies

Authenticode identifies the signer and protects the executable's signed content against
tampering. The certificate must be authorized for code signing, valid under Windows
Authenticode policy and chained to a publicly trusted CA. A certificate table alone does
not establish validity. Self-signed certificates, Public Trust Test and Private Trust
profiles are unsuitable for this public release route. Do not install custom trusted
roots to make a failed signature pass. See Microsoft's
[Smart App Control signing guidance](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control).

Smart App Control is Windows execution control using app intelligence and code integrity;
trusted publisher signing is the selected trust route when an application lacks an
established intelligence verdict. Enforcement applies when its setting is **On**, not
merely Evaluation. Read the
[Smart App Control overview](https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/overview).

SmartScreen checks downloaded-file and publisher reputation. A new signed EXE can still
produce an unrecognized-app warning, including with an EV certificate. A valid signature
does not guarantee approval by SmartScreen, Defender, Norton, enterprise policy or every
Smart App Control decision. See
[SmartScreen reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation).

## Release ordering and fail-closed behavior

The production sequence is:

```text
reviewed source -> validation -> native Tauri release build -> signing
-> timestamp-aware EXE verification -> prepare candidate distribution
-> hash final signed bytes -> resources/product.json -> create SHA-256 product.cat
-> sign and timestamp product.cat -> verify catalog, members and EXE
-> install verified candidate as root PLC Lab.exe + resources/
-> verify installed distribution -> native smoke -> publish signed artifact
```

The native Tauri executable is signed directly. It remains the sole application EXE,
named `PLC Lab.exe` after staging. Renaming and copying preserve the signed bytes; no
resource editing, version stamping, rebuilding or compression modifies the executable
after signing. Any such change requires signing again and regenerating the manifest.
`product.json` records release/signing evidence and the complete-file hash after signing.
It is authenticated through the detached Windows catalog `resources/product.cat`, signed
by the same configured trusted publisher with SHA-256 and an RFC 3161 timestamp. The
catalog covers the final EXE, `product.json`, listed resources and dependencies, and the
fallback BAT when shipped. It excludes itself to avoid recursive hashing. Verification
checks both the catalog signature and each required file's catalog membership; editing
the JSON and recomputing its hashes cannot manufacture publisher authorization.

The complete candidate is prepared and verified before root promotion. Catalog creation,
signing or verification failure preserves the previous installed release. Installation
uses a rollback transaction for the EXE and its matching resources. The catalog remains
a portable file; it is not registered in a Windows system catalog database and does not
change local certificate trust or Code Integrity policy. Microsoft documents the SDK
[MakeCat tool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/makecat), whose
version 2 catalogs support SHA-256 and are signed separately after creation.

Production commands fail with **SIGNING NOT CONFIGURED** or
**SIGNATURE VERIFICATION FAILED** when prerequisites or verification fail. They do not
quietly publish an unsigned release. Unsigned builds are explicitly development artifacts
and may be blocked by Smart App Control. Unsigned staging cannot silently overwrite an
existing signed production executable.

## SSL.com account and publisher setup

Complete the following using the actual developer's authorized account; no purchase
or identity submission has been made by this task:

1. Confirm with SSL.com that **IV Code Signing with eSigner for Code**, for an individual
   resident in Poland, meets the intended unattended Windows EXE and CAT signing use.
   Use the official IV product and pricing pages above; obtain a current quote.
2. Create an SSL.com account in the real full legal name and country **Poland**. Select
   the IV code-signing certificate and **eSigner cloud** storage, not document signing,
   OV requiring an organization, or an unnecessary hardware token.
3. Complete the provider's identity validation through its secure account interface.
   Use a current accepted identity document, the requested liveness/verification flow
   and any required address evidence. Review the name and country before approving.
   The provider may request more evidence or refuse issuance. Do not put identity
   documents or account credentials in this repository or chat.
4. After issuance, enroll the code-signing certificate in eSigner, activate the selected
   signing subscription, and configure the provider's supported automated signing
   credential/TOTP enrollment. Record the credential ID and exact issued certificate
   subject and public CA root chain. See the
   [eSigner FAQ](https://www.ssl.com/faqs/esigner-faq/) and
   [automation setup](https://www.ssl.com/how-to/integrating-esigner-with-ci-cd-pipelines-a-complete-setup-and-configuration-guide/).
5. Place the account username, password, signing credential ID and signing TOTP secret
   only in the protected GitHub environment described below. TOTP enrollment is a
   signing privilege; protect and rotate it along with the password. Do not share a
   production signing account with unrelated projects or publishers.
6. Configure the exact issued publisher and independently verified public root allowlist,
   then run a reviewed signed release and the required Windows device acceptance.
   No amount of checked-in automation substitutes for these external prerequisites.

The selected integration invokes a hash-pinned official **CodeSignTool** client directly
and captures its raw output so credentials cannot be echoed by a third-party action.
The provider's [supported file types](https://www.ssl.com/guide/esigner-supported-file-types/)
include both EXE and CAT. The repository still verifies the resulting SHA-256 signature
and RFC 3161 timestamp with Windows/SignTool; a provider success message is insufficient.
The first credentialed EXE/CAT release has not been run, so provider interoperability
remains unverified until onboarding and a successful production run.

## Signing configuration

Use the supplied nonsecret configuration example for your provider —
`config/windows-signing.example.json` (SSL.com eSigner),
`config/windows-signing.azure.example.json` (Azure Artifact Signing) or
`config/windows-signing.certificate-store.example.json` (a certificate in the Windows
store) — copy it to an ignored file such as `config/windows-signing.local.json`, and set
`PLC_LAB_SIGNING_CONFIG` to its absolute path.
Replace every example value. Never infer the expected publisher/root solely from an
unknown EXE.

| Field                    | Required value                                                                                                                |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| `provider`               | `ssl-esigner` for the selected service; alternatives are `azure-artifact-signing` and `certificate-store`                     |
| `publisherSubject`       | Exact full X.500 subject of the real issued publisher, as Windows reports it                                                  |
| `trustedRootThumbprints` | Explicit allowlist of authentic public root certificate thumbprints from the CA's official repository, verified independently |
| `signtoolPath`           | Optional absolute Windows SDK SignTool path; otherwise discovered from installed SDKs                                         |

For SSL.com, use its [official CA repository](https://www.ssl.com/repository/) to identify
and verify the actual issued chain's public root. A thumbprint identifies a certificate;
it does not select SHA-1 as the signing algorithm. Production signatures and catalog
hashes use SHA-256. A provider chain rotation requires a reviewed allowlist update.
The configuration JSON accepts no account passwords, TOTP secrets, private keys or tokens.

Install the supported Windows SDK x64 **SignTool and MakeCat**. Verification/installation
needs no signing account access. Signing additionally needs the selected provider's
supported client/authentication. The SSL.com workflow installs its pinned client and
passes protected secrets only to the signing subprocess.

For optional local production signing, install a trusted Java 17+ runtime, set
`JAVA_HOME` if needed, and install the pinned provider client with
`node scripts/esigner.mjs install`. Run `node scripts/esigner.mjs doctor` before adding
credentials: it checks the genuine pinned client/JVM without an account or signing call.
The client download/hash/extraction passed locally, while local `doctor` is blocked by
the absence of Java 17+. Standard hosted Windows validation installs the client and runs
this credential-free check with the runner's Java runtime. Supply the four `ESIGNER_*` secret values below
through a secure process environment, then run `node scripts/esigner.mjs preflight`
and `node scripts/windows-signing.mjs preflight`. Do not write secret values to commands
committed in Git or paste them in chat. Managed CI is the preferred way to obtain the
signed release; signature verification and installation need only nonsecret configuration.

For a CA certificate-store route, configure `certificateStore.thumbprint`,
`certificateStore.storeLocation` (`CurrentUser` or `LocalMachine`) and the CA's documented
RFC 3161 `certificateStore.timestampUrl`. Provision the exact public code-signing
certificate and private-key access using its supported HSM/token/cloud provider.
The repository does not import PFX files, install roots, enroll HSMs or manage token PINs.

### Azure alternative for an eligible organization

Azure is **not the setup path for this Polish individual**. An eligible organization may
use the implemented `azure-artifact-signing` adapter after obtaining a paid subscription,
completed public identity validation and an active **Public Trust** profile. Free/trial/
sponsored subscriptions are unsupported, and publisher names must be real validated
legal identities. See the [Artifact Signing FAQ](https://learn.microsoft.com/en-us/azure/artifact-signing/faq).

That adapter uses `azure.endpoint`, `azure.accountName`,
`azure.certificateProfileName` and a local x64 `azure.dlibPath` for signing. A supported
client setup uses .NET 8, the Microsoft Artifact Signing client tools and Azure CLI
authentication, as described in the
[official integration guide](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations).
For future Azure CI, use a profile-scoped signing principal and narrowly scoped protected
environment federation with OIDC; see [Microsoft OIDC](https://learn.microsoft.com/en-us/azure/developer/github/connect-from-azure-openid-connect)
and [profile-scoped roles](https://learn.microsoft.com/en-us/azure/artifact-signing/tutorial-assign-roles).
Do not enable broad PR/branch signing federation. The production workflow selected for
the current individual uses SSL.com instead of Azure.

## Protected GitHub production workflow

The trusted entry point is `.github/workflows/windows-signed-release.yml`, **Signed Windows
production release**. Its primary trigger is an explicitly pushed tag matching
**`plc-lab-production-*`** in `Michael-projeCtSS/PLC-Lab`. A read-only source gate requires
the tagged commit to be an ancestor of the reviewed remote
`origin/claude/product-development-v2` before validation or signing. It accepts no arbitrary
source/artifact input. Review and protect the development branch and workflow file.

`workflow_dispatch` remains available for the trusted development branch after the
workflow is registered on the default branch. The repository's default branch `main`
does not currently contain this workflow, so manual dispatch cannot be relied on now.
GitHub documents the [default-branch requirement for manual workflows](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).
The release-tag path works without changing `main` or replacing the active development
branch.

GitHub Environment **`windows-production-signing`** has been created with required reviewer
**Michael-projeCtSS**, a custom deployment-branch restriction to
`claude/product-development-v2`, the production tag pattern `plc-lab-production-*`, and
`PLC_LAB_SIGNING_READY=false`.
Self-review is allowed so the sole individual publisher can authorize a manual release;
this is a publisher approval gate, without an independent second reviewer. Review the
source/workflow before approving. Publisher secrets/configuration are still absent.
Keep the branch restriction and reviewer protection when adding credentials, and grant
no signing secrets to `pull_request` or `pull_request_target` workflows.

Configure the following only in that protected environment:

| Kind     | Name                          | Value                                                                                                                                  |
| -------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Secret   | `ESIGNER_USERNAME`            | SSL.com signing account username                                                                                                       |
| Secret   | `ESIGNER_PASSWORD`            | Signing account password                                                                                                               |
| Secret   | `ESIGNER_CREDENTIAL_ID`       | Exact eSigner credential ID for the issued IV code-signing certificate                                                                 |
| Secret   | `ESIGNER_TOTP_SECRET`         | Provider-authorized signing automation TOTP seed                                                                                       |
| Variable | `PLC_LAB_SIGNING_READY`       | `true` only after identity, certificate, subscription and environment protections are ready                                            |
| Variable | `PLC_LAB_SIGNING_CONFIG_JSON` | Nonsecret JSON: `provider: "ssl-esigner"`, exact `publisherSubject`, verified public `trustedRootThumbprints`, optional `signtoolPath` |

The JSON must contain no passwords, TOTP seed or tokens. The workflow creates a temporary
verification configuration. SSL.com's supported account/TOTP authentication is used for
this route; **no OIDC capability is claimed**. The signing job has no Azure token
permission. A leaked TOTP seed together with the password can authorize signing; keep
both inside the protected environment, avoid raw provider logs/artifacts and rotate
compromised credentials through the provider.

After account setup, secrets, nonsecret publisher/root configuration and
`PLC_LAB_SIGNING_READY=true`, choose a **full reviewed, validated commit SHA** already
pushed on `claude/product-development-v2`. Create a unique production tag at that exact
commit and push only that tag. Replace both placeholder values below:

```powershell
$releaseCommit = '<full-reviewed-and-validated-commit-sha>'
$releaseTag = 'plc-lab-production-<unique-version>'
git tag $releaseTag $releaseCommit
git push origin "refs/tags/$releaseTag"
```

Pushing this tag explicitly requests a production release and can consume signing
credits after approval. Do not create release tags on unreviewed code or move/reuse a
published tag. Inspect the run's full source SHA and validation results, then approve
the `windows-production-signing` environment as the publisher. No production tag has
been created or signing run requested by this task.

Canonical Windows/Linux validation runs at the immutable tagged source SHA. The production job verifies
the originating run, source SHA, immutable unsigned artifact ID and archive digest before
signing. After protected-environment approval it signs the exact application with the
pinned official client, prepares the post-signing manifest/catalog, signs the catalog,
verifies both signatures and all required members, installs the candidate at the root,
runs native smoke, re-verifies and uploads the signed distribution. Ordinary validation
artifacts remain explicitly **UNSIGNED development** builds with no signing access.

## Build and obtain the root executable

Development/debug remains available through `npm run dev` or `npm run build:desktop`.
To validate and stage an explicitly unsigned development build, use
`npm run build:release:unsigned`. Standard Windows and Linux CI retain unsigned validation
and do not receive signing privileges.

`BUILD_PLC_LAB_RELEASE.bat` and `npm run build:release` are production commands: they require
configured trusted signing, validate the source, build, sign, verify and stage the root
EXE and signed catalog. `npm run stage:release` requires production verification;
`npm run stage:release:unsigned` is explicitly for development. Use
`npm run check:distribution` for production verification and
`npm run check:distribution:unsigned` for the development inventory check.

For managed CI signing, run the protected Windows production release workflow after
configuration. Download the successful **signed** artifact for the intended commit from
GitHub Actions and extract it into a separate directory. Check the workflow's exact
source SHA and signature evidence. Close PLC Lab and install with:

```powershell
$env:PLC_LAB_SIGNING_CONFIG = (Resolve-Path -LiteralPath '.\config\windows-signing.local.json').Path
node scripts/install-signed-release.mjs 'C:\path\to\extracted-signed-distribution'
```

The installer verifies the EXE and catalog signatures, publisher, catalog membership and
file hashes before replacing
the repository's `PLC Lab.exe` and matching `resources/`. It checks the installed result.
This is the supported path to the required root location; leaving an unsigned root copy
while only uploading a signed CI artifact does not satisfy local installation acceptance.
Keep the EXE and resources from the same signed release together.

## Verify independently

In PowerShell, from the repository root:

```powershell
$signature = Get-AuthenticodeSignature -LiteralPath '.\PLC Lab.exe'
$signature | Format-List Status,StatusMessage,SignerCertificate,TimeStamperCertificate
Get-FileHash -LiteralPath '.\PLC Lab.exe' -Algorithm SHA256
node scripts/windows-signing.mjs verify '.\PLC Lab.exe'
npm run check:distribution
```

Also use the installed Windows SDK SignTool:

```powershell
& 'C:\path\to\Windows SDK\x64\signtool.exe' verify /pa /all /tw /v '.\PLC Lab.exe'
& 'C:\path\to\Windows SDK\x64\signtool.exe' verify /pa /all /tw /v '.\resources\product.cat'
& 'C:\path\to\Windows SDK\x64\signtool.exe' verify /pa /c '.\resources\product.cat' '.\resources\product.json'
```

Require `Valid`, the configured exact legal publisher, a verified trusted timestamp and
successful policy validation. Compare the full-file SHA-256 to
`resources/product.json` and the retained release evidence. SignTool `/pa` selects
Authenticode policy; `/all` checks every signature; `/tw` diagnoses missing timestamps.
The repository verifier also rejects a missing timestamp and checks every required
catalog member, not only `product.json`. See
[SignTool reference](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool).

## Timestamp, expiration and revocation

Signing uses SHA-256 for both file and timestamp digests, with RFC 3161 timestamping.
SSL.com documents `http://ts.ssl.com` as its RFC 3161 service, with
`/fd SHA256 /tr http://ts.ssl.com /td SHA256` for SignTool; see its
[signing instructions](https://www.ssl.com/how-to/using-your-code-signing-certificate/).
The eSigner client signs and timestamps through its provider configuration; the repository
independently requires the resulting signature and timestamp algorithms to be SHA-256.
An unexpected, missing or invalid timestamp fails the release.

For Artifact Signing use Microsoft's documented endpoint
`http://timestamp.acs.microsoft.com`, through `/fd SHA256 /tr ... /td SHA256`. The provider
documentation recommends this endpoint; it is not a made-up URL. Artifact Signing leaf
certificates last three days, so a proper timestamp is essential. See the
[signing integration instructions](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-signing-integrations).

A trusted timestamp establishes that signing occurred during certificate validity, so
normal signer-certificate expiration does not by itself invalidate an intact signature.
Use timestamp-aware Windows Authenticode verification; a raw certificate-chain check at
today's date is insufficient. See
[Authenticode timestamping](https://learn.microsoft.com/en-us/windows/win32/seccrypto/time-stamping-authenticode-signatures).

Monitor publisher identity and certificate lifecycle notifications. If identity renewal
expires, renew it through the provider before new signing. If a certificate/key is
revoked or compromised, stop releases, revoke access, contact the provider and investigate
the affected versions. Do not assume timestamping overrides revocation or a negative
security verdict. Rebuild clean reviewed source with the replacement authorized identity,
sign again, regenerate manifests and withdraw affected artifacts where appropriate.

## Diagnose a blocked signed executable

1. Record the exact EXE path, SHA-256, Windows version, detection text and time. Confirm
   the tested file is the installed signed root executable, not a stale development copy.
2. Run both verification commands above; retain signer/timestamp subjects, chain and
   revocation errors. Check the configured publisher and final-byte manifest. Do not fix
   a mismatch by editing the manifest to fit an unknown binary.
3. Inspect **Event Viewer > Applications and Services Logs > Microsoft > Windows >
   CodeIntegrity > Operational** around that time. Preserve events such as 3077 and 3033
   with the actual blocked path and any related signing information. The blocked module
   may differ from the main EXE.
4. Check Windows Security protection history and the active antivirus's detection
   history. Use the relevant vendor's false-positive/submission or support process with
   the exact signed file/hash when appropriate. A successful hosted smoke cannot settle
   an endpoint policy block.
5. Correct the actual issue: wrong artifact, broken signature, revoked identity, missing
   trust updates, timestamp/network validation failure or a security detection. Never
   disable Smart App Control/antivirus, edit Code Integrity policy, change security
   registry settings or import a custom root as a workaround.

## Smart App Control device acceptance

The release cannot claim complete Smart App Control acceptance until a Windows 11 device
whose Smart App Control setting remains **On** tests the actual signed root EXE. Record
the device/OS, source SHA, final EXE hash, publisher, signature/timestamp verification,
normal launch, second launch/single-instance behavior, knowledge resources and absence of
a new Code Integrity block for the application/modules. Keep the release and relocated
native smoke evidence as separate checks. Hosted runners may have no enforced Smart App
Control policy.

Current result: **SMART APP CONTROL DEVICE ACCEPTANCE: NOT VERIFIED**. The current unsigned
EXE cannot be used to fabricate a signed-device PASS.

## Remaining external prerequisites

- Confirm SSL.com's current quote and approval path for the developer's real Polish
  individual identity, purchase **IV Code Signing plus eSigner cloud signing**, and
  complete the provider's identity verification. No purchase has been made here.
- Obtain an issued public code-signing certificate, exact publisher subject, verified
  public chain, active signing subscription, credential ID and authorized automation/TOTP
  enrollment. Publisher identity is still unknown until issuance.
- Complete the already-created protected GitHub environment with the SSL.com secrets
  and exact nonsecret publisher/root settings. Keep the reviewed branch restrictions
  and required reviewer. Set `PLC_LAB_SIGNING_READY=true` only after identity, subscription
  and all configuration are ready; it is currently `false`.
- Run a successful signed EXE/catalog release, install its verified distribution at the
  root path and perform the enforced Smart App Control device acceptance test.

**Exact next user action:** obtain/confirm the SSL.com IV + eSigner quote and complete
account/identity onboarding through SSL.com's own site. Do not send the signing password
or TOTP secret in chat. After issuance, enter the documented protected environment
settings and run the release workflow.

No PFX, private key, certificate password, signing TOTP seed, Azure client secret or
access token belongs in Git. Production signing credentials must never be exposed to
untrusted pull-request code.
