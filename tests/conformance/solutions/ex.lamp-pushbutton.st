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

Lamp1 := Button1;
Lamp2 := AutoMode;

END_PROGRAM
