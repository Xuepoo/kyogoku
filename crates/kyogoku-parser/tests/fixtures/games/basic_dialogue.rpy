# This is a comment
define e = Character("Eileen")

label start:
    scene bg room
    show eileen happy

    # Basic dialogue
    e "You've created a new Ren'Py game."

    e "Once you add a story, pictures, and music, you can release it to the world!"

    # Narration
    "This is a narration line."

    # Multiline string
    """
    This is a multiline string
    spanning multiple lines.
    """

    menu:
        "It's a story.":
            jump story
        "It's a game.":
            jump game

    return
