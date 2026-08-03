# G19

## Struttura comunicazioni
- login: `{"action":"login", "username":"vehicle_1", "password":"password_1"}`
- register: `{"action":"register", "username":"vehicle_1", "password":"password_1"}`
- scelta modalità (-> le modalità possono andare in contemporanea? Se sì, le statistiche devono essere su journey già finiti e non su quello corrente): `{"mode":"tracking"}` oppure `{"mode":"statistics", "statistic":"main_speed"}` (Dobbiamo decidere se l'utente può selezionare una statistica alla volta o quando sceglie di visualizzarle le vede tutte basate sull'intervallo di tempo scelto e il journey)
- coordinate: `{"lat": 45.4642, "lon": 9.1900, "pos_time": "2026-08-02T11:07:00Z"}`

## Tabelle nel db
### tabella journeys

la tabella dei journey è composta da:

- `id -> INTEGER PRIMARY KEY`
- `user_id -> INTEGER NOT NULL`
- `lat -> REAL NOT NULL`
- `lon -> REAL NOT NULL`
- `pos_time -> DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP`

In questa tabella il journey_id serve per fare distinzione tra i vari journey che l'user potrebbe fare (anche quelli che hanno stesse coordinate) di conseguenza bisogna fare in modo cheil journey_id nella tabella venga incrementato ogni volta che viene creato un nuovo journey. Proprio per questo è necessario:

- definire motivo una variabile (thread-safe usando un Mutex) che tiene contiene il valore dell'id dell'ultimo journey creato
- se si vuole creare un nuovo journey si incrementa questa variabile e poi viene assegnato questo valore all'id del nuovo journey

E per l'interazione con il db quindi alla funzione di creazione di un nuovo record nella tabella **journeys** servono journey_id, user_id, latatitudine, longitudine, e pos_time (momento in qui l'user si trovava in quella posizione). Infatti per questo viene definita una struct **journey_waypoint** (definita in models) che contenga queste informazioni.

**Il client oltre a mandare le coordinate deve segnare l'stante delle coordinate e poi mandare tutto!** -> se questa cosa la dovesse fare il server (registrare il tempo in cui arrivano le coordinate) sarebbe istante della posizione + latenze per la trasmissione del messaggio
