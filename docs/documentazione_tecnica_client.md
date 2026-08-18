# Documentazione tecnica client

In questo documento verrá spiegata l'architettura ed il design del modulo client di Georust.

Moduli:
- `main.rs`: orchestratore centrale
- `config.rs`: gestore della configurazione
- `auth.rs`: autenticazione verso il server
- `coord_gen.rs`: generazione delle coordinate da inviare
- `console.rs`: gestore della console interattiva

Crate utilizzati:
- `tokio`
- `tokio tungstenite`
- `clap`
- `serde`

## Flusso di esecuzione

### Inizializzazione e connessione

Appena il binario client viene eseguito, vengono letti ed impostati i seguenti valori di configurazione:
- `coord_file_path`
- `tick_interval_millis`
- `client_username`
- `client_password`
- `server_url`

Questi file sono inizialmente letti dal file `client_config.json` e dai parametri di linea di comando (utilizzando il crate `clap`), i quali hanno priorità. Viene poi inizializzato l'oggetto `generator` che permetterà a `main` di ottenere in sequenza le coordinate da inviare. Dopo aver concluso l'operazione, il client si connette al server, ricevendo uno stream in lettura/scrittura e la risposta.

### Autenticazione

Prima di poter scambiare messaggi con il server, il client deve autenticarsi: le funzioni relative sono contenute nel modulo `auth.rs` che va ad esporre un unica funzione:

```rust
pub async fn authenticate(
    cfg: &Config,
    ws_write: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    ws_read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<i64, Box<dyn std::error::Error>>
```

Che accetta come input la configurazione e gli stream di lettura/scrittura ottenuti dalla fase di connessione.

### Invio messaggi verso il server

Una volta autenticato, il client è pronto ad inviare messaggi al server, sotto forma di strutture json condivise tra i binari. Vengono avviati due nuovi task asincroni `writer_task` e `reader_task` per la scrittura/lettura sullo stream websocket.

Le coordinate sono lette da un file tramite il modulo `coord_gen.rs`, che espone le funzioni di inizializzazione e lettura della coordinata attuale:

```rust
pub fn init(path: &str) -> io::Result<Self>
pub fn get_next(&mut self) -> Option<Coordinates>
```

Il metodo `get_next` accoppiato all'oggetto `ticker` sono utilizzati in un `loop` che consente di inviare la coordinata letta ad invervalli di tempo regolari, stabilita dal campo di configurazione `cfg.tick_interval_millis`. Il messaggio viene dunque comunicato al `writer_task` che si occupa dell'invio verso il server.

### Console interattiva

Mentre il processo client è in esecuzione, è disponibile all'utente una console interattiva che permette di inviare/ricevere messaggi testuali dal server, mettere in pausa l'invio di coordinate oppure terminare il processo. Questa logica é gestita da `console.rs`, un task asincrono avviato da `main.rs` che sfrutta cloni di canali preesistenti per la comunicazione con il task primario (tramite `event_tx`) ed il server (tramite `out_tx`):

```rust
pub async fn run(
    out_tx: mpsc::Sender<Message>,
    event_tx: mpsc::Sender<ConsoleEvent>
    )
```

Il modulo è un semplice parser che utilizza un `BufReader` per leggere `tokio::io::stdin()`, dove l'utente andrà a scrivere i comandi, per poi essere gestiti singolarmente da `pub fn handle_event(event: ConsoleEvent, sending: &mut bool) -> LoopControl`. Il tipo di ritorno `LoopControl` è una struct che va a dichiarare se il `loop` del `main` debba continuare: viene utilizzato infatti dal comando `exit` per terminare il processo.
