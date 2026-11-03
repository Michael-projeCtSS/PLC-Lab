PROGRAM Main
VAR_INPUT
    Button1 : BOOL; (* Pushbutton S1: starts a new shift count. *)
    Button2 : BOOL; (* Pushbutton S2 (unused). *)
    AutoMode : BOOL; (* Selector S3 in position AUTO (unused). *)
    PartPresent : BOOL; (* Proximity sensor B1 detects a part. *)
END_VAR
VAR_OUTPUT
    Lamp1 : BOOL; (* Green lamp H1: three or more parts counted. *)
    Lamp2 : BOOL; (* Amber lamp H2 (unused). *)
    Lamp3 : BOOL; (* Red lamp H3 (unused). *)
END_VAR
VAR RETAIN
    Count : INT := 0; (* Parts counted in this shift, kept over a warm restart. *)
END_VAR
VAR
    Edge : R_TRIG;
END_VAR

Edge(CLK := PartPresent);
IF Edge.Q THEN
    Count := Count + 1;
END_IF;
IF Button1 THEN
    Count := 0;
END_IF;
Lamp1 := Count >= 3;
Lamp2 := FALSE;
Lamp3 := FALSE;

END_PROGRAM
