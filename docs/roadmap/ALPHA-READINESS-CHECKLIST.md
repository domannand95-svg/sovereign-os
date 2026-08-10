# Sovereign OS Internal Alpha Readiness Checklist

Status: Active commissioning checklist  
Owner: @domannand95  
Updated: 2026-08-07

## Workstation and engineering environment

- [x] Windows 11 development host
- [x] Ubuntu 24.04 under WSL 2
- [x] Docker Desktop with WSL 2 engine
- [x] Rust 1.97.1 pinned toolchain
- [x] Rust nightly toolchain for fuzzing
- [x] Clippy, rustfmt, LLVM coverage tools, and Rust source component
- [x] Git, Git LFS, GitHub remote, and isolated worktrees
- [x] Python 3.14 engineering runtime
- [x] Python 3.11 transcription runtime
- [x] Ollama and Open WebUI local AI environment
- [x] cargo-audit, cargo-deny, cargo-llvm-cov, and cargo-fuzz
- [x] Trivy, Syft, and Cosign

## Verified baseline

- [x] Formatting, compilation, strict Clippy, and workspace tests pass
- [x] RustSec advisory scan reports no vulnerable dependencies
- [x] Trivy reports no high or critical dependency, secret, or configuration findings
- [x] Line coverage exceeds 90 percent
- [x] CycloneDX software bill of materials can be generated
- [ ] Repository licence selected and declared by the owner
- [ ] Decoder fuzz targets implemented and exercised
- [ ] Reproducible packaged-node acceptance test implemented
- [ ] Independent clean-machine release reproduction completed
- [ ] Release artifact signing policy approved and exercised

## Hardware currently available

- [x] Lenovo ThinkPad P16s, model 21BUS04A00
- [x] 32 GB memory
- [x] Internal Samsung 512 GB SSD
- [x] Intel Iris Xe graphics
- [x] NVIDIA T550 discrete graphics
- [x] SMART Board software stack
- [ ] SMART Board connected and hardware-validated
- [ ] Offline external SSD connected and designated

## Hardware and accessories to bring or acquire

### SMART Board commissioning

- Board power cable
- HDMI or DisplayPort video cable suitable for the board and laptop/dock
- USB touch/data cable; video alone does not provide touch or pen input
- Any required USB-C, HDMI, DisplayPort, or legacy board adapter
- SMART pens and eraser, if the model uses removable accessories
- Audio cable only if the model cannot receive audio over HDMI/DisplayPort
- Power board or surge-protected extension lead where required

### Recommended near-term productivity hardware

- 2–4 TB external SSD for governed offline backup and archive staging
- Reliable USB-C/Thunderbolt dock with power delivery, video, Ethernet, and USB ports
- Wired Ethernet cable for large repository, container, and archive transfers
- Quality USB headset or microphone for meetings and Whisper transcription
- Second conventional monitor if the SMART Board will not remain continuously connected

### Future infrastructure, only when operationally justified

- Dedicated development/AI workstation with a higher-memory NVIDIA GPU
- Network-attached storage with snapshots and controlled access
- Uninterruptible power supply for workstation and storage
- Dedicated local AI server after workload evidence demonstrates need

## Full SMART Board workflow reserved for physical commissioning

1. Detect the board and record model, serial number, cable path, and firmware.
2. Configure display mode, resolution, scaling, and audio output.
3. Orient and test touch, multitouch, pen, eraser, and long-press/right-click.
4. Annotate VS Code, PDFs, web pages, architecture diagrams, and SMART Notebook.
5. Use Open WebUI with a local Ollama model on the board.
6. Record speech and transcribe it locally through FFmpeg and Whisper.
7. Convert reviewed board notes into a bounded Sovereign OS task.
8. Implement in an isolated branch, run the acceptance gate, and review the evidence on the board.
9. Publish only after owner approval and record the final commissioning evidence.

## Internal alpha boundary

Internal alpha means the implemented single-node foundation can be exercised by
the owner in controlled environments. It does not mean unimplemented audit,
discovery, distributed, observability, packaging, or production-hardening
capabilities may be represented as complete.
