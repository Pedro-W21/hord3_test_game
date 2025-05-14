# Idée générale

Horde shooter avec du multi

carte carrée avec un objectif à défendre au centre

X minutes de préparation où on peut poser des murs et tout

3 niveaux de mur :
 - cassables
 - cassables mais se répare automatiquement à chaque vague
 - très difficiles à casser et se répare automatiquement à chaque vague

blocs piège :
 - DOT
    - piques
    - magma
    - électrique
 - tourelle
    - standard
    - rapide
    - perce 



   
# Comment faire du multijoueur

- TID = total ID, indique si c'est le monde, ou ent1, ou ent2
- INT = intéraction, contient un event
- le serveur garde une `HashMap<TID, Vec<INT>>`, la "intermap" pour chaque interaction
- nombre de tps fixe
- chaque packet est marqué de quel tick il vient
- générateurs de random synchronisés entre serveur/clients pour générer des ID
   - chaque ID sert à voir si les hash des champs exacts sont identiques, et si les champs à incertitudes sont proches
   - si y'a une différence, le serveur renvoie tout ce qui doit être synchronisé pour les N derniers ticks avec les intermap correspondantes
- toujours tester les entités de joueurs aussi
- tag d'event dans le protocole pour dire que ça vient d'un joueur (donc à passer à tout le monde)

# concepts plus précis

## Dungeon Crawler 9000

- équipe de 4
- donjon généré aléatoirement (peut être des voxels)
- étage par étage (les étages font pas forcément sens thématiquement ala backrooms)
- chaque salle doit être finie avant d'ouvrir la porte à une autre
- minimap dans l'UI
- level up d'équipe avec de l'XP par salle
- 




## Comment ça marche les voxels

- empty : 0b00XXXXXX
- pour tourner, on applique d'abord le 00, puis le 000
   - application du 00
      - LUT pour savoir où chaque bit du empty se swap
   - application du 000