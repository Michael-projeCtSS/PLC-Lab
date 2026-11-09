(* A reusable toggle: each press of Press flips On. *)
FUNCTION_BLOCK Toggle
VAR_INPUT
    Press : BOOL; (* The pushbutton. *)
END_VAR
VAR_OUTPUT
    On : BOOL; (* The toggled state. *)
END_VAR
VAR
    Edge : R_TRIG;
END_VAR

Edge(CLK := Press);
IF Edge.Q THEN
    On := NOT On;
END_IF;
END_FUNCTION_BLOCK

(* TRUE when both inputs are TRUE. *)
FUNCTION Both : BOOL
VAR_INPUT
    A : BOOL;
    B : BOOL;
END_VAR
Both := A AND B;
END_FUNCTION

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
    Toggle1 : Toggle; (* One instance for S1 and H1. *)
    Toggle2 : Toggle; (* One instance for S2 and H2. *)
END_VAR

Toggle1(Press := Button1, On => Lamp1);
Toggle2(Press := Button2, On => Lamp2);
Lamp3 := Both(Lamp1, Lamp2);

END_PROGRAM
