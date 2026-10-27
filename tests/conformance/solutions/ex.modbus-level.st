PROGRAM Main
VAR_INPUT
    StartPB : BOOL; (* Start button S1 (normally open). *)
    StopPB : BOOL; (* Stop button S2 (normally closed). *)
    LevelHighHigh : BOOL; (* Independent high-high switch at 90 % (TRUE when wet). *)
    LevelPermille : INT; (* Level from Modbus holding register 0 (40001), 0..1000 = 0..100 %. *)
    LevelPercent : REAL; (* Level from holding registers 1-2 (40002-40003), IEEE 754 float, high word first. *)
    LevelCommOK : BOOL; (* TRUE while the Modbus scan of the transmitter succeeds. *)
END_VAR
VAR_OUTPUT
    InletValve : BOOL; (* Inlet valve solenoid XV-101. *)
    DrainPump : BOOL; (* Drain pump P-102. *)
END_VAR
VAR
    Filling : BOOL;
END_VAR

Filling := (StartPB OR Filling) AND StopPB AND LevelCommOK AND NOT LevelHighHigh AND LevelPermille < 800;
InletValve := Filling;
DrainPump := FALSE;

END_PROGRAM
