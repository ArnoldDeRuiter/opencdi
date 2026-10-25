# Brain Dead 13 (CD-i) - Reverse Engineering & Architecture Documentation

Questo documento raccoglie in modo esaustivo tutte le deduzioni, analisi del disassemblato, flussi di segnali OS-9 e interazioni hardware CDIC/DVC ricavate durante il lavoro su **Brain Dead 13 (USA)** per OpenCDi.

---

## 1. Mappa della Memoria e Moduli

Durante l'esecuzione su CD-i 220 (CPU SCC68070, OS-9 CD-RTOS):
- **Modulo Principale `cdi_bd13`:** Caricato in memoria a partire da `0x00DE1080`.
  - Base dati / frame pointer: `A6 = 0x00D08000`.
  - Strutture globali e code eventi allocate a offset negativi rispetto ad `A6` (es. `-$7000..-$8000(a6)`).
- **Driver CD-Audio/Play `cdapdriv`:** Caricato a `0x0007B530`. Gestisce lo streaming dei settori del disco, i buffer CDIC, i trigger e gli allarmi.
- **Driver DVC `vmpega.rom`:** Caricato a `0x00E40000`. Gestisce la cartuccia MPEG-1 (C-Cube CL450) per il video motion e l'audio MPEG Layer 2.
- **Video Bumper Iniziale (`misc.rtf`):** Filmato introduttivo Philips Media a schermo pieno, riprodotto dal frame 0 al frame 1675.

---

## 2. Flusso di Esecuzione e Segnali al Termine del Logo

### A. Fine della riproduzione di `misc.rtf`
1. Al **Frame 1675, Linea 30**, la cartuccia DVC (`vmpega.rom`) completa lo svuotamento del bitstream del filmato `misc.rtf`.
2. Il driver DVC invia il segnale OS-9 `0x5210` (End of Stream) al processo `cdi_bd13`.
3. Nel signal intercept handler di `cdi_bd13` (`0xDF1C56`):
   - La ricezione del segnale `0x5210` accoda l'evento interno `0x100A` (Advance Scene / Next Scene):
     ```m68k
     0xDF1C56: HIT PUSH Event 0x100A
     ```

### B. Il Dispatcher degli Eventi
Al **Frame 1677, Linea 48**, il dispatcher degli eventi consuma `0x100A`:
- `0xDF11EE: addq.w #1, -$711a(a6)` incrementa l'indice della scena corrente `cur_711a` da 0 a 1.
- `0xDF1200: tst.b -$7020(a6)` testa se la Scena 1 (`LOGOMENU`) è già stata precaricata in RAM:
  - Se il byte `-$7020(a6) == 0`: la scena **non** è precaricata.
  - Salta a `0xDF1242`, accoda l'evento `0x100B`, e prosegue in `0xDF1276..0xDF1446`.

### C. La Causa del Freeze (Schermo Nero Permanente)
All'indirizzo `0xDF1276`:
- Il codice legge il descrittore della Scena 1 dal vettore delle scene:
  ```m68k
  desc = *(-$7ebc(a6) + scene_idx * 4); // scene_idx = 1
  ```
- Poiché la Scena 1 non è stata precaricata, il puntatore `desc` è `0x00000000` (NULL).
- A `0xDF1446..0xDF1474`, il gioco dereferenzia `*(desc + 0x2C)` $\rightarrow$ legge a `0x0000002C` (vettore trap/eccezione del 68070 in low memory), ottenendo puntatori non-null casuali (`0x00000034`).
- A `0xDF1474..0xDF149A`, un ciclo `while (p != NULL)` scorre la memoria bassa cercando un terminatore null in una linked list fittizia. Poiché la memoria contiene vettori e codice, il loop non termina mai (`i = 1, 2, ... 20000+` iterazioni per scanline), saturando la CPU e bloccando l'emulatore su schermo nero.

---

## 3. Come Viene Precaricata la Scena 1

Nel normale funzionamento:
1. La Scena 1 (`LOGOMENU`) deve essere precaricata prima della fine del logo.
2. La routine di caricamento della scena si trova a `0xDF3794` (`load_scene`), invocata dall'evento `0x200C` a `0xDF18C8`:
   ```m68k
   0xDF18C8: cmpi.w #$200c, d0
   ...
   0xDF18EE: bsr.w $df3794        ; Carica la scena
   0xDF1904: move.l d0, (a0, d1.l) ; Salva descrittore in -$7ebc(a6) + scene_idx * 4
   ```
3. L'evento `0x200C` viene generato quando `cdapdriv` invia il segnale `0x200C` (o `0x200D` a `0xDF1946`).
4. `cdapdriv` invia questo segnale in due modalità:
   - **Trigger di Settore:** All'arrivo di un settore specifico con bit submode Trigger (`0x40`) impostato.
   - **Allarme Temporizzato:** Tramite `F$Alarm` asincrono schedulato durante lo streaming.

---

## 4. Analisi Hardware CDIC nel Driver `cdapdriv`

Il disassemblato approfondito di `cdapdriv.bin` ha rivelato i controlli hardware esatti eseguiti sul chip CDIC:

### A. Registro CDIC `0x3FFE` (`DBUF`)
- In `cdapdriv` a `0x0EE6` e `0x305A`:
  ```m68k
  0EE6: move.b $1(a4), d1     ; Legge byte basso del registro CDIC 0x3FFE (DBUF)
  0EEA: btst.b #$5, d1        ; Testa Bit 5 (0x0020)
  0EEE: beq.w $f08
  0EF2: move.w #$914, d0      ; Se Bit 5 è 1, imposta dimensione Mode 2 Form 2 (2324 byte)
  0EF6: move.b #$0, $1c6(a2)  ; Resetta flag errore $1c6
  ```
- **Bit 5 (`0x0020`) di `DBUF`:**
  - Indica che il settore ricevuto nel buffer CDIC è valido, decodificato correttamente e privo di errori EDC/ECC.
  - Se il bit 5 non è impostato, il driver presume che il settore sia corrotto o non pronto, impostando il flag `$1c6(a2) = 2`.

### B. Offset `0x09B8` nella RAM CDIC
- In `cdapdriv` a `0x29CA` e `0x3066`:
  ```m68k
  29CA: move.w $9b8(a3), d1
  29CE: andi.w #$7ff, d1
  29D2: beq.w $2a46           ; Se d1 & 0x7FF == 0: NESSUN ERRORE, procede normalmente!
  29D6: move.w #$f4, d1       ; Altrimenti errore E$Read (0xF4)
  29DA: move.w d1, $96(a2)
  ```
  - Offset `0x09B8` nella RAM CDIC corrisponde ai flag di errore/status hardware del settore.
  - Il valore `0` indica che il settore è integro (0 errori). Se non nullo, attiva la routine di recupero errori `0x304E` (`F$SRqMem` memoria tampone).

### C. Trigger di Settore (`Submode 0x40`)
- A `0x29A8..0x29B2`:
  ```m68k
  29A8: move.w $a(a3, d2.w), d0  ; Byte 22 del settore (Submode)
  29AC: lsr.w #$8, d0
  29AE: andi.b #$40, d0          ; Bit 6 = Trigger
  29B2: beq.w $29f6
  ```
- Se il bit 5 di `DBUF` non è presente o il trigger viene scartato prematuramente, il preload della Scena 1 non viene schedulato, determinando il fallimento della transizione post-logo.

---

## 5. Stato Attuale e Prossimi Passi

1. **Stato Attuale:**
   - La riproduzione audio/video di `misc.rtf` (Philips Media bumper) è perfetta al 100%.
   - Isolato l'indirizzo esatto del blocco (`0xDF1474`) e la causa (descrittore Scena 1 NULL per mancato preload).
   - Verificate le specifiche CDIC per il bit 5 (`0x0020`) del registro `0x3FFE`.

2. **Intervento da completare per Brain Dead 13:**
   - In [`opencdi/src/cdic.rs`](file:///c:/Users/Gian/.gemini/antigravity/scratch/philips%20cdi/opencdi/src/cdic.rs), in `process_sector()`:
     - Assicurare che `self.data_buffer` mantenga attivo il bit 5 (`0x0020`) durante la ricezione di settori validi: `self.data_buffer |= 0x4020;`.
     - Verificare la corretta notifica del trigger PCB a `cdapdriv` in modo che il segnale `0x200C` raggiunga `cdi_bd13` prima del Frame 1675.
   - Verificare che Space Ace e gli altri titoli non subiscano regressioni con questa modifica.
