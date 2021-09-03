# bevy-sandbox
Repo for experimenting with bevy engine

# Design

"Actions" are asserted without regard for any other entity.

"Effects" are registered by consensus of all applicable entities.

For example, any entity may assert a `MoveAction`, but `MoveEffect` is the move that will actually be acted out after collision detection.

```
keyboard input --keyboard_system--> MoveAction --collision_system--> MoveEffect
```

For animation it may look like
```
keyboard_input --keyboard_system--> MoveAction, PlayerAction --animate_system--> AnimationEffect
```