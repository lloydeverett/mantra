# Mantra

A desktop ritual: type a short mantra, commit to what you'll do next, then do it for a timed session before the next mantra.

## Language

**Mantra**:
A short line from Marcus Aurelius that must be typed exactly (case ignored) to proceed.

**Round**:
One pass through the ritual: a mantra, a commitment, and a session.

**Commitment**:
What the user promises to do for the session, stated in their own words (e.g. "Wash the dishes").
_Avoid_: Task, intention, goal

**Commitment prompt**:
The step after the mantra where the user states a commitment, picks a duration and any tags, within a time limit.

**Session**:
The timed period in which the user carries out their commitment.
_Avoid_: Rest, break

**Duration**:
How long a session lasts, from 1 minute to 4 hours. Either typed into the commitment, picked from a preset, or the default.

**Auto**:
The duration choice that uses the duration typed in the commitment, or the default if none was typed.

**Tag**:
A fixed, built-in marker a commitment can carry, written `@name` in the commitment (e.g. `@work`, `@personal`). A commitment can carry several; they are not part of its words.
_Avoid_: Label, category, context

**Waiting**:
The part of a round from the mantra's appearing until the session starts.

**While Waiting policy**:
What happens to the screens while waiting, picked from a menu: None, Dim (the default: the screens darken until the mantra's typed, and again as the session nears its end), or Hide (every other app's windows are hidden until the session starts). Not kept across launches.
_Avoid_: Mode, effect

**Snooze**:
A stretch of time, picked from a menu, in which the While Waiting policy is held off: the screens aren't dimmed and other windows aren't hidden. The round carries on as normal underneath, and the window counts the snooze down until it ends or is cancelled.
