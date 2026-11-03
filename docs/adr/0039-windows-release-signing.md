# ADR-0039: Trusted Authenticode for Windows production releases

- Status: Accepted for the explicitly requested Windows signing task; external publisher setup remains incomplete
- Date: 2026-11-03
- Extends: ADR-0030 (one root application executable)

## Context

The root `PLC Lab.exe` is the Tauri application, but the audited local build is unsigned.
Compilation and hosted native smoke do not establish Windows publisher trust. Signing
changes executable bytes, so the existing executable hash must be computed afterward.

## Decision

Production releases require a validated, timestamped Authenticode signature whose
publisher matches explicit configuration. The user confirmed an individual developer in
Poland; Azure Public Trust is unavailable to that account type/location. Select **SSL.com
IV Code Signing with eSigner for Code** through a pinned official CodeSignTool client,
pending purchase, identity validation and credential provisioning. The provider accepts
Poland as a registrant country and supports individual validation; actual approval is
external and is not assumed.

Azure Artifact Signing **Public Trust** with narrowly scoped OIDC remains an alternative
for an eligible organization, and a supported public certificate-store/HSM provider is
the other adapter. No self-signed or private-trust route is accepted for public
production distribution. The SSL.com route uses protected account/TOTP credentials;
there is no claim that its supported authentication provides OIDC.

The production sequence is validation, native release build, EXE signing, cryptographic
verification, candidate preparation, final-byte manifest generation, SHA-256 Windows
catalog creation, catalog signing, distribution verification, transactional root
installation, native smoke and artifact publication. `resources/product.cat` authenticates
`product.json`, the signed EXE, resources, dependencies and the optional fallback BAT;
it excludes itself to avoid recursion. Both EXE and catalog require the configured trusted
publisher and an RFC 3161 timestamp. The catalog is portable and is never installed into
the system catalog database.

Copying or renaming the signed executable preserves its bytes. The complete candidate
must pass signature, catalog membership and inventory verification before the root is
changed; installation rolls back on failure and verifies the final root distribution
again. An unsigned development build is labeled explicitly and cannot silently replace
a signed production copy. The Windows SDK must supply both SignTool and MakeCat.

Signing authority is confined to a separate trusted release workflow and protected GitHub
environment, with no pull-request signing. Missing configuration and verification errors
fail the production release. Identity verification, paid service setup and protected
environment configuration are external prerequisites, not implied by checked-in code.

Production tags use `plc-lab-production-*` and must point to a reviewed commit on the
remote development branch; a read-only ancestry gate precedes validation and signing.
The protected environment allows only that tag pattern and the development branch, with
the individual publisher approving releases. This provides a usable release trigger
while the workflow is absent from the repository's default `main` branch, without
changing `main`. Manual dispatch remains optional after default-branch registration.

## Alternatives

Commercial OV/EV signing is valid when the selected CA accepts the legal publisher and
offers a compatible key provider. EV does not buy an unconditional SmartScreen bypass.
Store/MSIX distribution does not preserve the requested standalone EXE delivery and is
only an alternative. Importing a self-signed root or weakening Windows security is
excluded by the task.

## Consequences

The executable remains `PLC Lab.exe`, with `resources/` beside it. PLC compilation,
runtime, simulation and educational semantics are unaffected. Local unsigned testing
remains possible but can be blocked by Windows security. Release automation cannot sign
until an eligible publisher supplies a trusted signing account or certificate. A passing
signature gate does not substitute for an actual Smart App Control enabled-device test.

See [Windows code signing](../WINDOWS_CODE_SIGNING.md) for setup, verification and the
initial executable audit.
