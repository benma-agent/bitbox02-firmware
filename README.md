# Monogram passphrase confirmation review images

Rendered from the production BitBox02 confirmation components.
Before: pull/master at 4f329b8249ac81a81a5902da461ef1d0bef5471b.
After: benma-agent/monogram-passphrase-confirmation.

The bundled Monogram TTF is generated at size 20 for a 12-row bitmap with
9-pixel capitals, matching the height of master's password_11X12 font.
Only the space bitmap is customized, using master's visible-space glyph.
Both keyboard picker fonts are unchanged from master.

- c.png: confirmation of "I like lilies1".
- g.png: confirmation of "O0o Iil1 |".
- s.png: confirmation of " I l 0  " (leading, trailing, and repeated spaces).

Master is on the left and Monogram on the right.
Screens contain exact 128x64 framebuffers enlarged 6x without smoothing.
Captions are outside the screens. These are software renderings, not device photos.
