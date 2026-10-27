PROGRAM Main
VAR_INPUT
    Button1 : BOOL; (* Pushbutton S1, normally open. *)
    Button2 : BOOL; (* Pushbutton S2, normally open. *)
    AutoMode : BOOL; (* Selector S3 in position AUTO. *)
    PartPresent : BOOL; (* Proximity sensor B1 detects a part. *)
END_VAR
VAR_OUTPUT
    Lamp1 : BOOL; (* Green lamp H1. *)
    Lamp2 : BOOL; (* Amber lamp H2. *)
    Lamp3 : BOOL; (* Red lamp H3. *)
END_VAR
VAR
    Parts : CTU;
    Jam : TON;
END_VAR

Lamp1 := AutoMode;
Parts(CU := PartPresent AND AutoMode, R := Button2, PV := 4);
Lamp2 := Parts.Q;
Jam(IN := PartPresent AND AutoMode, PT := T#2s);
Lamp3 := Jam.Q;

END_PROGRAM
