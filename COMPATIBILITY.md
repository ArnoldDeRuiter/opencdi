# Game Compatibility List - OpenCDi

[Versione Italiana](COMPATIBILITY_IT.md)

This document tracks the compatibility status, test results, and verified fixes for Philips CD-i titles on **OpenCDi**, focusing on hardware fidelity for MCD-212 plane mixing, CDIC ADPCM audio, and the Digital Video Cartridge (DVC / MPEG-1).

---

## Compatibility Summary Table

| # | File / Title | DVC Required | Current Status | Key Notes |
|---|---|:---:|:---:|---|
| 1 | `Hotel Mario (USA).chd` | No | **Playable / Excellent** | CD-DA and ADPCM audio are clear; accurate Plane A/B graphics; responsive controls and stable cutscenes. |
| 2 | `Dragon's Lair (USA).chd` | **Yes** | **Playable / Excellent** | Smooth full-motion MPEG-1 video; Dirk autonomous input fixed via authentic resting pad neutral coordinates. |
| 3 | `Lost Eden (Europe).chd` | **Yes** | **Playable / Very Good** | Synchronized FMV video and CD-DA/ADPCM audio; smooth environment transitions. |
| 4 | `7th Guest, The (USA).chd` | **Yes** | **Playable / Very Good** | FMV audio distortion resolved; post-ghost cutscene corridor freeze resolved via decoupled FMA stream underflow handling. |
| 5 | `Striker Pro (USA).chd` | No | **Playable** | 2D dual-plane graphics rendered accurately; in-game match playable. |
| 6 | `Rise of the Robots (Europe).chd` | No | **Bootable** | Intro sequences and menus functional. |
| 7 | `alberoazzurro.chd` | **Yes** | **Bootable / Very Good** | DVC intro video and cutscenes rendered via MCD-212 EV1/EV2 plane routing in `mix_lines`. |
| 8 | `Chaos Control (USA).chd` | **Yes** | **Bootable / Excellent** | DVC cartridge detected; FMV intro and interactive main menu operational. |
| 9 | `Creature Shock (Europe) (Disc 1).chd` | **Yes** | **Bootable / Excellent** | DVC cartridge detected; space FMV cutscene smooth; Disc 2 hot-swap supported without system reset. |
| 10 | `Dark Fables of Aesop The (Europe).chd` | No / TBD | *Pending testing* | Added to test suite, pending automated verification. |
| 11 | `Escape from CyberCity (USA).chd` | **Yes** | **Bootable / Excellent** | DVC detected; Start Game / Exit / Tour main menu fully functional. |
| 12 | `Inca (Europe).chd` | **Yes** | **Bootable / Excellent** | DVC detected; language selection screen and cutscenes operational. |
| 13 | `International Tennis Open (Europe).chd` | No | *Pending testing* | Added to test suite, pending automated verification. |
| 14 | `Kingdom - The Far Reaches (USA).chd` | **Yes** | **Bootable / Excellent** | DVC detected (missing cartridge warning resolved); animated FMV MPEG-1 intro video smooth. |
| 15 | `Mutant Rampage - Bodyslam (USA).chd` | **Yes** | **Bootable / Excellent** | Boot hang resolved with native PAL boot sequence for European BIOS; DVC detected; Start/Setup menus working. |
| 16 | `Tangram - The Ultimate Chinese Game (Europe).chd` | No | *Pending testing* | Added to test suite, pending automated verification. |
| 17 | `Ultra CD-i Soccer (Europe).chd` | **Yes** | **Bootable / Excellent** | DVC detected (missing cartridge warning resolved); title screen and pitch rendering functional. |
| 18 | `Space Ace (USA).chd` | **Yes** | **Playable / Excellent** | Full-motion MPEG-1 video and audio fully functional; gameplay interactions verified. |
| 19 | `Brain Dead 13 (USA) (Disc 1).chd` | **Yes** | **Intro Functional / Under Investigation** | Philips Media FMV bumper (`misc.rtf`) video and audio play smoothly; post-logo scene 1 preload under diagnosis. |

---

## Detailed Test Notes

### 1. Hotel Mario (USA)
- **DVC:** Not required (native MCD-212 Plane A/B rendering).
- **Status:** Fully playable.
- **Applied Fixes:**
  - Accurate CLUT palette management and DCA/ICA register sequencing to eliminate flickering.
  - Synchronized Slave MCU gamepad interrupts and CDIC ADPCM audio processing.

### 2. Dragon's Lair (USA)
- **DVC:** Required (Digital Video Cartridge with MPEG-1 video and Audio Layer 2).
- **Status:** Fully playable.
- **Applied Fixes:**
  - Eliminated premature CDIC streaming abort on track/scene jumps.
  - Calibrated DVC timer clock and GOP/SEQ interrupt signaling.

### 3. Lost Eden (Europe)
- **DVC:** Required.
- **Status:** Playable.
- **Applied Fixes:**
  - Restored proper End-of-Stream (EOS) detection on MPEG bitstream buffer drain.
  - Guarded FMV timer against spurious interrupt triggers while decoder is inactive.

### 4. The 7th Guest (USA)
- **DVC:** Required.
- **Status:** Fully playable / corridor freeze resolved.
- **Applied Fixes:**
  1. **Black screen post-video:** Resolved. Removed incorrect `DCR_DE_BIT` gate on Path 1 in `mcd212.rs`, restoring 3D room backdrops (Plane B) and interactive hotspots (Plane A).
  2. **Upstairs hallway post-ghost cutscene freeze:** Resolved. Fully decoupled FMV video and FMA audio streams, implemented proper ISO `00 00 01 B9` program end routing, and prevented premature `FMA_IRQ_EOI` assertions during momentary audio buffer drain.
  3. **Cutscene-end black flash / flicker elimination:** Resolved. Authentic Philips CD-i / DVC hardware retains the last decoded MPEG picture in DVC DRAM until overwritten. Replaced premature framebuffer clearing on stream drain/stop with persistent frame scanout (`has_valid_frame`), providing smooth visual continuity during disc seeks into the interactive mansion.

### 5. Ultra CD-i Soccer (Europe)
- **DVC:** Required.
- **Status:** Bootable and fully functional title screen / pitch.
- **Applied Fixes:**
  - Corrected `effective_active_scanlines()` in `mcd212.rs` to authentic Motorola/Philips MCD-212 hardware specifications (Table 5-6). Removed erroneous checks on `DCR_CF_BIT` (Cursor Flag) and `CSR1W_ST_BIT` that clamped PAL active scanlines to 240 instead of 280. This allows scanline 289 to execute Path 1 DCA `INTERRUPT` (`0x60000000`), triggering the Level 5 IRQ needed by OS-9's video callback and resolving the infinite spin loop at `0x25E694`.


### 5. Striker Pro (USA)
- **DVC:** Not required.
- **Status:** Bootable and playable.

### 6. Rise of the Robots (Europe)
- **DVC:** Not required.
- **Status:** Bootable (intro and menus).

---

## Video CD (White Book / CD-i Bridge) Support

| # | Title | CUE Format | Current Status | Technical Highlights |
|---|---|:---:|:---:|---|
| V1 | `CD-i See It Hear It Feel It (USA) (Kilby Predicts) (Rev 1)` | Multi-file (`Track 1.bin`, `Track 2.bin`, `Track 3.bin`) | **Playable / Excellent** | Continuous MPEG-1 decoding from Track 2 (`AVSEQ01.DAT`, LBA 81480+), KILBY velvet curtain intro in 11,187 colors, 0 underflow, synchronized audio (850+ frames verified). |
| V2 | `Gateway to the Mind` | Single-file multi-track (`Gateway to the Mind.bin`) | **Bootable / Very Good** | Proper boot into `CDI/CDI_VCD.APP`, smooth playback of animated CD-i bumper `CDI_BUM.DAT` (LBA 862..1507) with MPEG-1 audio/video and Plane B backdrop display. |

---

## Titles Queued for Verification
The following titles are in the game library and queued for automated test runner passes:
- `Dark Fables of Aesop The (Europe).chd`
- `International Tennis Open (Europe).chd`
- `Tangram - The Ultimate Chinese Game (Europe).chd`
