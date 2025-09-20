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

## How to do voxel collision
- do other kinds of collision first in main tick and add to speed
- do voxel checks in after main
- if any point of the AABB is inside terrain :
   - compute the "speed nudge" needed to get it to an empty tile quickest in all directions
   - compare all nudges, take the smallest one in all directions (in absolute terms) that makes the collider no longer collide with anything
   - give priority to up/down, if only an up/down nudge works, do that

## Experiment concept : Agent playground
- every entity is an "agent"
- agents have :
   - collider
   - inventory
   - movement characteristics
      - jump height
      - movement speed
   - "Planning AI"
      - pathfinding/moving through path
         - A* through integer grid
         - start at integered origin, explore in integer direction towards the objective
         - if that has already been explored, explore in all directions until you can explore towards the objective
         - have HashSet of all positions that have been explored (as in a path already goes through there, or it's impassable)
         - make a tree inside a vec
            - each node has a parent, tried directions (with their node ids) and 
      - aiming/shooting
      - keeps track of 
   - "Actions"
      - list of "actions" like :
         - jumping
         - moving in a direction
         - turning
         - placing/breaking block
         - using an item
      - if an action can be done/tried directly, do it (e.g. jumping)
      - if it requires planning, ask "planning AI" for extra actions
      - each action has an ID, so the Director can know when a multi-step action has ended
      - an action can be given a deadline/timeout (in game ticks)
      - an action can either :
         - End (was possible to try, and tried)
         - timeout (wasn't done in time)
   - "Director"
      - provides actions to perform
      - can be a player, an LLM, an expert system, nothing, a loop...
      - LLM Director
         - how to provide world information to an LLM ?
            - grid of nearby tiles for sight, à la old roguelikes
            - (x,y) grid where x are numbers and y are letters
            - 3 grids provided, a layer below, the entity's layer, and a layer above (layers are numbered -1, 0, 1)
            - characters used for world description :
               - `,` : solid ground (can stand there)
               - `.` : empty (would fall there)
               - `%` : solid block on same level (can't go through that)
               - `a` : other agent on same level
               - `@` : agent
            - describe who each other agent is by (coordinates) : name
         - how can an LLM act ?
            - actions as function calls :
               - format : same as Proxima (needs to be evolved so it works better)
               - possible actions :
                  - SAY {text}
                  - 
         - how to connect LLMs to the engine ?
            - agents have prompt response vecs and request ID generators
            - extra_data contains a Sender<HordeProximaRequest>
            - HordeProximaRequest :
               - request id
               - source entity (TID)
               - Prompt
            - another thread handles talking to Proxima
               - that thread has the Receiver<HordeProximaRequest>
               - has a Sender<HordeProximaResponse>
                  - contains the TID and request ID, as well as the response
            - the main thread (or any tick-synchronized thread) has the Receiver<HordeProximaResponse>
               - each tick, receives and creates events filling the prompt vecs of the relevant entities


# Multiplayer improvements :
- the client controls a few IDs that he refuses resets and events for 
- server sets tickrate, clients slow down when necessary