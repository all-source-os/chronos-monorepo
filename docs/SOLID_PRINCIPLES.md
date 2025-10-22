# SOLID Principles Guide for AllSource Event Store

**Version**: 1.0
**Last Updated**: 2025-10-21
**Author**: AllSource Core Team

---

## 📋 Table of Contents

1. [Introduction](#introduction)
2. [Single Responsibility Principle (SRP)](#single-responsibility-principle-srp)
3. [Open/Closed Principle (OCP)](#openclosed-principle-ocp)
4. [Liskov Substitution Principle (LSP)](#liskov-substitution-principle-lsp)
5. [Interface Segregation Principle (ISP)](#interface-segregation-principle-isp)
6. [Dependency Inversion Principle (DIP)](#dependency-inversion-principle-dip)
7. [SOLID in Practice](#solid-in-practice)
8. [Anti-Patterns](#anti-patterns)
9. [Refactoring Guide](#refactoring-guide)

---

## Introduction

### What are SOLID Principles?

SOLID is an acronym for five design principles that make software designs more understandable, flexible, and maintainable:

- **S**ingle Responsibility Principle
- **O**pen/Closed Principle
- **L**iskov Substitution Principle
- **I**nterface Segregation Principle
- **D**ependency Inversion Principle

### Why SOLID for Event Stores?

Event stores require:
- **Flexibility**: Swap storage engines without breaking code
- **Testability**: Test components in isolation
- **Maintainability**: Add features without breaking existing code
- **Performance**: Optimize critical paths independently

SOLID principles help achieve all of these goals.

---

## Single Responsibility Principle (SRP)

> **A class/module should have one, and only one, reason to change.**

### What It Means

Each module should have a single, well-defined responsibility. If a class has multiple responsibilities, they become coupled, and changes to one responsibility may affect others.

### Rust Examples

#### ❌ Violating SRP

```rust
// Bad: Event has too many responsibilities
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub payload: Value,
    // ...
}

impl Event {
    // Responsibility 1: Domain logic
    pub fn is_valid(&self) -> bool {
        !self.event_type.is_empty()
    }

    // Responsibility 2: Serialization
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    // Responsibility 3: Persistence
    pub async fn save_to_database(&self, db: &Database) -> Result<()> {
        db.execute("INSERT INTO events...").await
    }

    // Responsibility 4: Validation
    pub fn validate_schema(&self, schema: &Schema) -> Result<()> {
        schema.validate(&self.payload)
    }

    // Responsibility 5: Notification
    pub async fn notify_subscribers(&self, notifier: &Notifier) -> Result<()> {
        notifier.send(&self).await
    }
}
```

**Problems**:
- Changes to database schema affect Event
- Changes to JSON format affect Event
- Changes to notification system affect Event
- Hard to test (too many dependencies)
- Violates SRP: 5 reasons to change!

#### ✅ Following SRP

```rust
// Good: Each struct has one responsibility

// Responsibility 1: Domain logic only
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub entity_id: String,
    pub tenant_id: String,
    pub payload: Value,
    pub timestamp: DateTime<Utc>,
}

impl Event {
    pub fn new(event_type: String, entity_id: String, tenant_id: String, payload: Value) -> Result<Self, DomainError> {
        // Domain validation only
        if entity_id.is_empty() {
            return Err(DomainError::InvalidEntityId);
        }

        Ok(Self {
            id: Uuid::new_v4(),
            event_type,
            entity_id,
            tenant_id,
            payload,
            timestamp: Utc::now(),
        })
    }

    pub fn belongs_to_tenant(&self, tenant_id: &str) -> bool {
        self.tenant_id == tenant_id
    }
}

// Responsibility 2: Serialization
pub struct EventSerializer;

impl EventSerializer {
    pub fn to_json(event: &Event) -> Result<String, SerializationError> {
        serde_json::to_string(event)
            .map_err(|e| SerializationError::JsonError(e.to_string()))
    }

    pub fn from_json(json: &str) -> Result<Event, SerializationError> {
        serde_json::from_str(json)
            .map_err(|e| SerializationError::JsonError(e.to_string()))
    }
}

// Responsibility 3: Persistence
pub struct EventRepository {
    db: Arc<Database>,
}

impl EventRepository {
    pub async fn save(&self, event: &Event) -> Result<(), RepositoryError> {
        self.db.execute("INSERT INTO events...").await
    }

    pub async fn find_by_id(&self, id: &Uuid) -> Result<Option<Event>, RepositoryError> {
        // Query database
        Ok(None)
    }
}

// Responsibility 4: Validation
pub struct EventValidator {
    schema_registry: Arc<SchemaRegistry>,
}

impl EventValidator {
    pub fn validate(&self, event: &Event) -> Result<(), ValidationError> {
        let schema = self.schema_registry.get(&event.event_type)?;
        schema.validate(&event.payload)
    }
}

// Responsibility 5: Notification
pub struct EventNotifier {
    subscribers: Vec<Arc<dyn Subscriber>>,
}

impl EventNotifier {
    pub async fn notify(&self, event: &Event) -> Result<(), NotificationError> {
        for subscriber in &self.subscribers {
            subscriber.on_event(event).await?;
        }
        Ok(())
    }
}
```

**Benefits**:
- ✅ Each struct has one reason to change
- ✅ Easy to test in isolation
- ✅ Easy to reuse (composition)
- ✅ Changes are localized

### Go Examples

#### ❌ Violating SRP

```go
// Bad: UserService does too much
type UserService struct {
    db          *sql.DB
    jwtSecret   string
    emailClient *EmailClient
    logger      *Logger
}

func (s *UserService) Register(username, email, password string) error {
    // Responsibility 1: Validation
    if username == "" || email == "" || password == "" {
        return errors.New("invalid input")
    }

    // Responsibility 2: Password hashing
    hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
    if err != nil {
        return err
    }

    // Responsibility 3: Database
    _, err = s.db.Exec("INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)",
        username, email, hash)
    if err != nil {
        return err
    }

    // Responsibility 4: Logging
    s.logger.Info("User registered: " + username)

    // Responsibility 5: Email
    return s.emailClient.SendWelcomeEmail(email)
}
```

#### ✅ Following SRP

```go
// Good: Each type has one responsibility

// Domain entity - only domain logic
type User struct {
    ID           string
    Username     string
    Email        string
    PasswordHash string
    CreatedAt    time.Time
}

func NewUser(username, email, passwordHash string) (*User, error) {
    if username == "" {
        return nil, errors.New("username cannot be empty")
    }
    if email == "" {
        return nil, errors.New("email cannot be empty")
    }

    return &User{
        ID:           uuid.New().String(),
        Username:     username,
        Email:        email,
        PasswordHash: passwordHash,
        CreatedAt:    time.Now().UTC(),
    }, nil
}

// Repository - only data access
type UserRepository struct {
    db *sql.DB
}

func (r *UserRepository) Save(ctx context.Context, user *User) error {
    _, err := r.db.ExecContext(ctx,
        "INSERT INTO users (id, username, email, password_hash, created_at) VALUES (?, ?, ?, ?, ?)",
        user.ID, user.Username, user.Email, user.PasswordHash, user.CreatedAt)
    return err
}

func (r *UserRepository) FindByUsername(ctx context.Context, username string) (*User, error) {
    // Query logic
    return nil, nil
}

// Password service - only password operations
type PasswordService struct{}

func (s *PasswordService) Hash(password string) (string, error) {
    hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
    if err != nil {
        return "", err
    }
    return string(hash), nil
}

func (s *PasswordService) Verify(password, hash string) error {
    return bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
}

// Email service - only email operations
type EmailService struct {
    client *EmailClient
}

func (s *EmailService) SendWelcomeEmail(email string) error {
    return s.client.Send(email, "Welcome!", "Thanks for registering!")
}

// Use case - orchestrates the above
type RegisterUserUseCase struct {
    userRepo        *UserRepository
    passwordService *PasswordService
    emailService    *EmailService
    logger          *Logger
}

func (uc *RegisterUserUseCase) Execute(ctx context.Context, username, email, password string) error {
    // Hash password
    hash, err := uc.passwordService.Hash(password)
    if err != nil {
        return err
    }

    // Create user
    user, err := NewUser(username, email, hash)
    if err != nil {
        return err
    }

    // Save user
    if err := uc.userRepo.Save(ctx, user); err != nil {
        return err
    }

    // Log
    uc.logger.Info("User registered: " + username)

    // Send email (async, don't block)
    go uc.emailService.SendWelcomeEmail(email)

    return nil
}
```

### Clojure Examples

#### ❌ Violating SRP

```clojure
;; Bad: One namespace does everything
(ns allsource.user
  (:require [clojure.java.jdbc :as jdbc]
            [buddy.hashers :as hashers]
            [postal.core :as postal]))

(defn register-user!
  "Registers a new user (does too much!)"
  [db username email password]

  ;; Responsibility 1: Validation
  (when (or (empty? username) (empty? email) (empty? password))
    (throw (ex-info "Invalid input" {})))

  ;; Responsibility 2: Password hashing
  (let [password-hash (hashers/derive password)]

    ;; Responsibility 3: Database
    (jdbc/insert! db :users
      {:username username
       :email email
       :password_hash password-hash
       :created_at (java.util.Date.)})

    ;; Responsibility 4: Logging
    (println "User registered:" username)

    ;; Responsibility 5: Email
    (postal/send-message {:host "smtp.example.com"}
      {:from "noreply@example.com"
       :to email
       :subject "Welcome!"
       :body "Thanks for registering!"})))
```

#### ✅ Following SRP

```clojure
;; Good: Separate namespaces for each responsibility

;; Domain entity
(ns allsource.domain.user
  (:require [clojure.spec.alpha :as s]))

(s/def ::username (s/and string? #(not (empty? %))))
(s/def ::email (s/and string? #(re-matches #".+@.+\..+" %)))
(s/def ::password-hash string?)

(s/def ::user
  (s/keys :req-un [::id ::username ::email ::password-hash ::created-at]))

(defn new-user
  "Create a new user with validation"
  [username email password-hash]
  (let [user {:id (java.util.UUID/randomUUID)
              :username username
              :email email
              :password-hash password-hash
              :created-at (java.util.Date.)}]
    (if (s/valid? ::user user)
      user
      (throw (ex-info "Invalid user" (s/explain-data ::user user))))))

;; Repository - database operations only
(ns allsource.infrastructure.user-repository
  (:require [clojure.java.jdbc :as jdbc]))

(defn save!
  "Save user to database"
  [db user]
  (jdbc/insert! db :users user))

(defn find-by-username
  "Find user by username"
  [db username]
  (jdbc/query db ["SELECT * FROM users WHERE username = ?" username]
              {:result-set-fn first}))

;; Password service - password operations only
(ns allsource.services.password
  (:require [buddy.hashers :as hashers]))

(defn hash-password
  "Hash a password"
  [password]
  (hashers/derive password))

(defn verify-password
  "Verify a password against hash"
  [password hash]
  (hashers/check password hash))

;; Email service - email operations only
(ns allsource.services.email
  (:require [postal.core :as postal]))

(defn send-welcome-email!
  "Send welcome email to new user"
  [email]
  (postal/send-message {:host "smtp.example.com"}
    {:from "noreply@example.com"
     :to email
     :subject "Welcome!"
     :body "Thanks for registering!"}))

;; Use case - orchestrates the above
(ns allsource.use-cases.register-user
  (:require [allsource.domain.user :as user]
            [allsource.infrastructure.user-repository :as repo]
            [allsource.services.password :as password]
            [allsource.services.email :as email]
            [clojure.tools.logging :as log]))

(defn execute!
  "Execute user registration use case"
  [db username email password]

  ;; Hash password
  (let [password-hash (password/hash-password password)

        ;; Create user entity
        user (user/new-user username email password-hash)]

    ;; Save to database
    (repo/save! db user)

    ;; Log
    (log/info "User registered:" username)

    ;; Send email (async)
    (future (email/send-welcome-email! email))

    user))
```

### How to Identify SRP Violations

Ask these questions:
1. Can I describe this class/module in one sentence without using "and"?
2. How many reasons does this code have to change?
3. If I change this code, will it affect multiple unrelated features?

---

## Open/Closed Principle (OCP)

> **Software entities should be open for extension but closed for modification.**

### What It Means

You should be able to add new functionality without changing existing code. Use abstractions (traits/interfaces/protocols) to allow new implementations.

### Rust Examples

#### ❌ Violating OCP

```rust
// Bad: Need to modify existing code to add new storage types
pub enum StorageType {
    Parquet,
    CSV,
    // If we want to add JSON, we must modify this enum!
}

pub struct EventStore {
    storage_type: StorageType,
}

impl EventStore {
    pub async fn save(&self, event: &Event) -> Result<()> {
        match self.storage_type {
            StorageType::Parquet => {
                // Parquet save logic
            }
            StorageType::CSV => {
                // CSV save logic
            }
            // To add JSON, we must modify this match statement!
        }
        Ok(())
    }
}
```

**Problem**: Every new storage type requires modifying EventStore!

#### ✅ Following OCP

```rust
// Good: Open for extension, closed for modification
pub trait EventStorage: Send + Sync {
    async fn save(&self, event: &Event) -> Result<(), StorageError>;
    async fn load(&self, id: &Uuid) -> Result<Option<Event>, StorageError>;
}

// Existing implementation
pub struct ParquetStorage {
    path: PathBuf,
}

impl EventStorage for ParquetStorage {
    async fn save(&self, event: &Event) -> Result<(), StorageError> {
        // Parquet implementation
        Ok(())
    }

    async fn load(&self, id: &Uuid) -> Result<Option<Event>, StorageError> {
        Ok(None)
    }
}

// Existing implementation
pub struct CsvStorage {
    path: PathBuf,
}

impl EventStorage for CsvStorage {
    async fn save(&self, event: &Event) -> Result<(), StorageError> {
        // CSV implementation
        Ok(())
    }

    async fn load(&self, id: &Uuid) -> Result<Option<Event>, StorageError> {
        Ok(None)
    }
}

// NEW: Can add JSON storage without modifying existing code!
pub struct JsonStorage {
    path: PathBuf,
}

impl EventStorage for JsonStorage {
    async fn save(&self, event: &Event) -> Result<(), StorageError> {
        // JSON implementation
        Ok(())
    }

    async fn load(&self, id: &Uuid) -> Result<Option<Event>, StorageError> {
        Ok(None)
    }
}

// EventStore is closed for modification
pub struct EventStore {
    storage: Arc<dyn EventStorage>,  // Works with any implementation!
}

impl EventStore {
    pub async fn save(&self, event: &Event) -> Result<()> {
        self.storage.save(event).await?;
        Ok(())
    }
}
```

**Benefits**:
- ✅ Add new storage types without modifying EventStore
- ✅ Existing code remains untouched
- ✅ Easy to test (mock EventStorage)

### Go Examples

#### ❌ Violating OCP

```go
// Bad: Must modify to add new notification types
type NotificationService struct {
    notificationType string
}

func (s *NotificationService) Send(message string) error {
    switch s.notificationType {
    case "email":
        // Email logic
        return sendEmail(message)
    case "slack":
        // Slack logic
        return sendSlack(message)
    // To add SMS, must modify this switch!
    default:
        return errors.New("unknown notification type")
    }
}
```

#### ✅ Following OCP

```go
// Good: Open for extension via interface
type Notifier interface {
    Send(message string) error
}

// Existing implementation
type EmailNotifier struct {
    smtpHost string
    smtpPort int
}

func (n *EmailNotifier) Send(message string) error {
    // Email implementation
    return nil
}

// Existing implementation
type SlackNotifier struct {
    webhookURL string
}

func (n *SlackNotifier) Send(message string) error {
    // Slack implementation
    return nil
}

// NEW: Can add SMS without modifying existing code!
type SMSNotifier struct {
    twilioSID   string
    twilioToken string
}

func (n *SMSNotifier) Send(message string) error {
    // SMS implementation
    return nil
}

// NotificationService is closed for modification
type NotificationService struct {
    notifiers []Notifier  // Can use any implementation!
}

func (s *NotificationService) Notify(message string) error {
    for _, notifier := range s.notifiers {
        if err := notifier.Send(message); err != nil {
            return err
        }
    }
    return nil
}
```

### Clojure Examples

#### ❌ Violating OCP

```clojure
;; Bad: Must modify function to add new formats
(defn export-events
  [events format]
  (case format
    :json (to-json events)
    :csv (to-csv events)
    :xml (to-xml events)
    ;; To add YAML, must modify this function!
    (throw (ex-info "Unknown format" {:format format}))))
```

#### ✅ Following OCP

```clojure
;; Good: Open for extension via protocols
(defprotocol EventExporter
  (export [this events] "Export events in specific format"))

;; Existing implementations
(defrecord JsonExporter []
  EventExporter
  (export [_ events]
    (json/generate-string events)))

(defrecord CsvExporter []
  EventExporter
  (export [_ events]
    (csv/write-csv (map to-csv-row events))))

(defrecord XmlExporter []
  EventExporter
  (export [_ events]
    (xml/generate-xml events)))

;; NEW: Can add YAML without modifying existing code!
(defrecord YamlExporter []
  EventExporter
  (export [_ events]
    (yaml/generate-string events)))

;; Export service is closed for modification
(defn export-events
  [exporter events]
  (export exporter events))  ;; Works with any exporter!

;; Usage
(export-events (->JsonExporter) events)
(export-events (->YamlExporter) events)  ;; No changes to export-events!
```

---

## Liskov Substitution Principle (LSP)

> **Objects of a superclass should be replaceable with objects of its subclasses without breaking the application.**

### What It Means

If S is a subtype of T, then objects of type T may be replaced with objects of type S without altering any of the desirable properties of the program.

### Rust Examples

#### ❌ Violating LSP

```rust
// Bad: ReadOnlyRepository violates LSP
trait EventRepository {
    async fn save(&self, event: Event) -> Result<()>;
    async fn load(&self, id: Uuid) -> Result<Option<Event>>;
}

struct ParquetRepository { /* ... */ }

impl EventRepository for ParquetRepository {
    async fn save(&self, event: Event) -> Result<()> {
        // Works as expected
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }
}

// Violates LSP: Can't actually save!
struct ReadOnlyRepository { /* ... */ }

impl EventRepository for ReadOnlyRepository {
    async fn save(&self, event: Event) -> Result<()> {
        // Unexpected behavior: Returns error instead of saving
        Err(anyhow::anyhow!("Read-only repository!"))
    }

    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }
}

// Code that expects EventRepository will break with ReadOnlyRepository!
async fn ingest_events(repo: &dyn EventRepository, events: Vec<Event>) {
    for event in events {
        repo.save(event).await.unwrap();  // Panics with ReadOnlyRepository!
    }
}
```

**Problem**: ReadOnlyRepository doesn't fulfill the contract - it can't save!

#### ✅ Following LSP

```rust
// Good: Separate traits for different capabilities
trait EventReader {
    async fn load(&self, id: Uuid) -> Result<Option<Event>>;
    async fn find_by_entity(&self, entity_id: &str) -> Result<Vec<Event>>;
}

trait EventWriter {
    async fn save(&self, event: Event) -> Result<()>;
    async fn save_batch(&self, events: Vec<Event>) -> Result<()>;
}

// Full repository implements both
struct ParquetRepository { /* ... */ }

impl EventReader for ParquetRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }

    async fn find_by_entity(&self, entity_id: &str) -> Result<Vec<Event>> {
        Ok(vec![])
    }
}

impl EventWriter for ParquetRepository {
    async fn save(&self, event: Event) -> Result<()> {
        Ok(())
    }

    async fn save_batch(&self, events: Vec<Event>) -> Result<()> {
        Ok(())
    }
}

// Read-only repository only implements reader
struct ReadOnlyRepository { /* ... */ }

impl EventReader for ReadOnlyRepository {
    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }

    async fn find_by_entity(&self, entity_id: &str) -> Result<Vec<Event>> {
        Ok(vec![])
    }
}
// Doesn't implement EventWriter - type system prevents misuse!

// Code is explicit about requirements
async fn query_events(reader: &dyn EventReader) {
    // Only reads - works with both ParquetRepository and ReadOnlyRepository
}

async fn ingest_events(writer: &dyn EventWriter, events: Vec<Event>) {
    // Only writes - works with ParquetRepository, NOT ReadOnlyRepository
    // Type system prevents passing ReadOnlyRepository!
}
```

### Go Examples

#### ❌ Violating LSP

```go
// Bad: Square violates LSP for Rectangle
type Rectangle interface {
    SetWidth(width int)
    SetHeight(height int)
    GetArea() int
}

type NormalRectangle struct {
    width  int
    height int
}

func (r *NormalRectangle) SetWidth(width int) {
    r.width = width
}

func (r *NormalRectangle) SetHeight(height int) {
    r.height = height
}

func (r *NormalRectangle) GetArea() int {
    return r.width * r.height
}

// Violates LSP: Changing width also changes height!
type Square struct {
    side int
}

func (s *Square) SetWidth(width int) {
    s.side = width  // Sets BOTH width and height!
}

func (s *Square) SetHeight(height int) {
    s.side = height  // Sets BOTH width and height!
}

func (s *Square) GetArea() int {
    return s.side * s.side
}

// This code breaks with Square!
func TestRectangle(r Rectangle) {
    r.SetWidth(5)
    r.SetHeight(10)
    expected := 50
    actual := r.GetArea()

    if actual != expected {
        // Fails with Square! (actual = 100)
        panic("Area should be 50!")
    }
}
```

#### ✅ Following LSP

```go
// Good: Separate interfaces for different shapes
type Shape interface {
    GetArea() int
}

type Rectangle struct {
    width  int
    height int
}

func (r *Rectangle) SetWidth(width int) {
    r.width = width
}

func (r *Rectangle) SetHeight(height int) {
    r.height = height
}

func (r *Rectangle) GetArea() int {
    return r.width * r.height
}

type Square struct {
    side int
}

func (s *Square) SetSide(side int) {
    s.side = side
}

func (s *Square) GetArea() int {
    return s.side * s.side
}

// Both implement Shape, each with appropriate methods
func CalculateTotalArea(shapes []Shape) int {
    total := 0
    for _, shape := range shapes {
        total += shape.GetArea()
    }
    return total
}
```

### Clojure Examples

#### ❌ Violating LSP

```clojure
;; Bad: InMemoryCache violates protocol contract
(defprotocol Cache
  (put [this key value ttl]
    "Store value with TTL in seconds")
  (get [this key]
    "Retrieve value by key"))

(defrecord RedisCache [connection]
  Cache
  (put [_ key value ttl]
    (redis/setex connection key ttl value))
  (get [_ key]
    (redis/get connection key)))

;; Violates LSP: Ignores TTL!
(defrecord InMemoryCache [store]
  Cache
  (put [_ key value ttl]
    ;; Ignores TTL - violates contract!
    (swap! store assoc key value))
  (get [_ key]
    (@store key)))
```

#### ✅ Following LSP

```clojure
;; Good: All implementations respect the contract
(defprotocol Cache
  (put [this key value ttl]
    "Store value with TTL in seconds")
  (get [this key]
    "Retrieve value by key"))

(defrecord RedisCache [connection]
  Cache
  (put [_ key value ttl]
    (redis/setex connection key ttl value))
  (get [_ key]
    (redis/get connection key)))

(defrecord InMemoryCache [store]
  Cache
  (put [_ key value ttl]
    ;; Properly implements TTL with scheduled removal
    (swap! store assoc key {:value value :expires-at (+ (System/currentTimeMillis) (* ttl 1000))})
    (future
      (Thread/sleep (* ttl 1000))
      (swap! store dissoc key)))
  (get [_ key]
    (when-let [entry (@store key)]
      (if (< (System/currentTimeMillis) (:expires-at entry))
        (:value entry)
        (do
          (swap! store dissoc key)
          nil)))))

;; Both implementations can be used interchangeably
(defn cache-user-session [cache user-id session ttl]
  (put cache user-id session ttl))  ;; Works with both!
```

---

## Interface Segregation Principle (ISP)

> **No client should be forced to depend on methods it does not use.**

### What It Means

Large interfaces should be split into smaller, more specific ones so that clients only need to know about the methods they use.

### Rust Examples

#### ❌ Violating ISP

```rust
// Bad: One large trait forces implementations to implement everything
trait EventStore: Send + Sync {
    // Read operations
    async fn load(&self, id: Uuid) -> Result<Option<Event>>;
    async fn query(&self, filter: Filter) -> Result<Vec<Event>>;

    // Write operations
    async fn save(&self, event: Event) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;

    // Snapshot operations
    async fn create_snapshot(&self, entity_id: &str) -> Result<Snapshot>;
    async fn load_snapshot(&self, entity_id: &str) -> Result<Option<Snapshot>>;

    // Analytics operations
    async fn aggregate(&self, query: AggregateQuery) -> Result<Aggregation>;
    async fn time_series(&self, query: TimeSeriesQuery) -> Result<TimeSeries>;

    // Replication operations
    async fn replicate_to(&self, target: &str) -> Result<()>;
    async fn sync_from(&self, source: &str) -> Result<()>;
}

// Problem: Simple read-only store must implement ALL methods!
struct ReadOnlyStore { /* ... */ }

impl EventStore for ReadOnlyStore {
    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        // Implemented
        Ok(None)
    }

    async fn query(&self, filter: Filter) -> Result<Vec<Event>> {
        // Implemented
        Ok(vec![])
    }

    // Forced to implement write operations (doesn't make sense!)
    async fn save(&self, event: Event) -> Result<()> {
        Err(anyhow::anyhow!("Read-only!"))
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        Err(anyhow::anyhow!("Read-only!"))
    }

    // Forced to implement snapshot operations (might not need!)
    async fn create_snapshot(&self, entity_id: &str) -> Result<Snapshot> {
        Err(anyhow::anyhow!("Not supported!"))
    }

    async fn load_snapshot(&self, entity_id: &str) -> Result<Option<Snapshot>> {
        Ok(None)
    }

    // Forced to implement analytics (might not need!)
    async fn aggregate(&self, query: AggregateQuery) -> Result<Aggregation> {
        Err(anyhow::anyhow!("Not supported!"))
    }

    async fn time_series(&self, query: TimeSeriesQuery) -> Result<TimeSeries> {
        Err(anyhow::anyhow!("Not supported!"))
    }

    // Forced to implement replication (might not need!)
    async fn replicate_to(&self, target: &str) -> Result<()> {
        Err(anyhow::anyhow!("Not supported!"))
    }

    async fn sync_from(&self, source: &str) -> Result<()> {
        Err(anyhow::anyhow!("Not supported!"))
    }
}
```

#### ✅ Following ISP

```rust
// Good: Small, focused traits
trait EventReader: Send + Sync {
    async fn load(&self, id: Uuid) -> Result<Option<Event>>;
    async fn query(&self, filter: Filter) -> Result<Vec<Event>>;
}

trait EventWriter: Send + Sync {
    async fn save(&self, event: Event) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}

trait SnapshotProvider: Send + Sync {
    async fn create_snapshot(&self, entity_id: &str) -> Result<Snapshot>;
    async fn load_snapshot(&self, entity_id: &str) -> Result<Option<Snapshot>>;
}

trait Analytics: Send + Sync {
    async fn aggregate(&self, query: AggregateQuery) -> Result<Aggregation>;
    async fn time_series(&self, query: TimeSeriesQuery) -> Result<TimeSeries>;
}

trait Replicator: Send + Sync {
    async fn replicate_to(&self, target: &str) -> Result<()>;
    async fn sync_from(&self, source: &str) -> Result<()>;
}

// Now implementations only implement what they need!
struct ReadOnlyStore { /* ... */ }

impl EventReader for ReadOnlyStore {
    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }

    async fn query(&self, filter: Filter) -> Result<Vec<Event>> {
        Ok(vec![])
    }
}
// That's it! No forced implementation of unused methods

// Full-featured store can implement multiple traits
struct FullEventStore { /* ... */ }

impl EventReader for FullEventStore { /* ... */ }
impl EventWriter for FullEventStore { /* ... */ }
impl SnapshotProvider for FullEventStore { /* ... */ }
impl Analytics for FullEventStore { /* ... */ }
impl Replicator for FullEventStore { /* ... */ }

// Clients depend only on what they need
async fn query_service(reader: &dyn EventReader) {
    // Only needs EventReader
}

async fn ingestion_service(writer: &dyn EventWriter) {
    // Only needs EventWriter
}
```

### Go Examples

#### ❌ Violating ISP

```go
// Bad: Fat interface
type EventStore interface {
    // Reads
    Load(id uuid.UUID) (*Event, error)
    Query(filter Filter) ([]*Event, error)

    // Writes
    Save(event *Event) error
    Delete(id uuid.UUID) error

    // Snapshots
    CreateSnapshot(entityID string) (*Snapshot, error)
    LoadSnapshot(entityID string) (*Snapshot, error)

    // Analytics
    Aggregate(query AggregateQuery) (*Aggregation, error)
    TimeSeries(query TimeSeriesQuery) (*TimeSeries, error)

    // Replication
    ReplicateTo(target string) error
    SyncFrom(source string) error
}

// Forced to implement everything!
type SimpleStore struct{}

func (s *SimpleStore) Load(id uuid.UUID) (*Event, error) { return nil, nil }
func (s *SimpleStore) Query(filter Filter) ([]*Event, error) { return nil, nil }
func (s *SimpleStore) Save(event *Event) error { return nil }
func (s *SimpleStore) Delete(id uuid.UUID) error { return nil }
func (s *SimpleStore) CreateSnapshot(entityID string) (*Snapshot, error) { return nil, errors.New("not supported") }
func (s *SimpleStore) LoadSnapshot(entityID string) (*Snapshot, error) { return nil, errors.New("not supported") }
func (s *SimpleStore) Aggregate(query AggregateQuery) (*Aggregation, error) { return nil, errors.New("not supported") }
func (s *SimpleStore) TimeSeries(query TimeSeriesQuery) (*TimeSeries, error) { return nil, errors.New("not supported") }
func (s *SimpleStore) ReplicateTo(target string) error { return errors.New("not supported") }
func (s *SimpleStore) SyncFrom(source string) error { return errors.New("not supported") }
```

#### ✅ Following ISP

```go
// Good: Segregated interfaces
type EventReader interface {
    Load(id uuid.UUID) (*Event, error)
    Query(filter Filter) ([]*Event, error)
}

type EventWriter interface {
    Save(event *Event) error
    Delete(id uuid.UUID) error
}

type SnapshotProvider interface {
    CreateSnapshot(entityID string) (*Snapshot, error)
    LoadSnapshot(entityID string) (*Snapshot, error)
}

type Analytics interface {
    Aggregate(query AggregateQuery) (*Aggregation, error)
    TimeSeries(query TimeSeriesQuery) (*TimeSeries, error)
}

// Simple store only implements what it needs
type SimpleStore struct{}

func (s *SimpleStore) Load(id uuid.UUID) (*Event, error) { return nil, nil }
func (s *SimpleStore) Query(filter Filter) ([]*Event, error) { return nil, nil }
func (s *SimpleStore) Save(event *Event) error { return nil }
func (s *SimpleStore) Delete(id uuid.UUID) error { return nil }
// That's it!

// Full store implements multiple interfaces
type FullStore struct{}

func (s *FullStore) Load(id uuid.UUID) (*Event, error) { return nil, nil }
func (s *FullStore) Query(filter Filter) ([]*Event, error) { return nil, nil }
func (s *FullStore) Save(event *Event) error { return nil }
func (s *FullStore) Delete(id uuid.UUID) error { return nil }
func (s *FullStore) CreateSnapshot(entityID string) (*Snapshot, error) { return nil, nil }
func (s *FullStore) LoadSnapshot(entityID string) (*Snapshot, error) { return nil, nil }
func (s *FullStore) Aggregate(query AggregateQuery) (*Aggregation, error) { return nil, nil }
func (s *FullStore) TimeSeries(query TimeSeriesQuery) (*TimeSeries, error) { return nil, nil }

// Clients depend only on what they need
func QueryService(reader EventReader) {
    // Only needs EventReader
}

func IngestionService(writer EventWriter) {
    // Only needs EventWriter
}
```

### Clojure Examples

#### ❌ Violating ISP

```clojure
;; Bad: One large protocol
(defprotocol EventStore
  (load-event [this id])
  (query-events [this filter])
  (save-event [this event])
  (delete-event [this id])
  (create-snapshot [this entity-id])
  (load-snapshot [this entity-id])
  (aggregate [this query])
  (time-series [this query])
  (replicate-to [this target])
  (sync-from [this source]))

;; Forced to implement everything!
(defrecord SimpleStore []
  EventStore
  (load-event [_ id] nil)
  (query-events [_ filter] [])
  (save-event [_ event] nil)
  (delete-event [_ id] nil)
  (create-snapshot [_ entity-id] (throw (ex-info "Not supported" {})))
  (load-snapshot [_ entity-id] nil)
  (aggregate [_ query] (throw (ex-info "Not supported" {})))
  (time-series [_ query] (throw (ex-info "Not supported" {})))
  (replicate-to [_ target] (throw (ex-info "Not supported" {})))
  (sync-from [_ source] (throw (ex-info "Not supported" {}))))
```

#### ✅ Following ISP

```clojure
;; Good: Segregated protocols
(defprotocol EventReader
  (load-event [this id])
  (query-events [this filter]))

(defprotocol EventWriter
  (save-event [this event])
  (delete-event [this id]))

(defprotocol SnapshotProvider
  (create-snapshot [this entity-id])
  (load-snapshot [this entity-id]))

(defprotocol Analytics
  (aggregate [this query])
  (time-series [this query]))

;; Simple store only implements what it needs
(defrecord SimpleStore []
  EventReader
  (load-event [_ id] nil)
  (query-events [_ filter] [])

  EventWriter
  (save-event [_ event] nil)
  (delete-event [_ id] nil))

;; Full store implements multiple protocols
(defrecord FullStore []
  EventReader
  (load-event [_ id] nil)
  (query-events [_ filter] [])

  EventWriter
  (save-event [_ event] nil)
  (delete-event [_ id] nil)

  SnapshotProvider
  (create-snapshot [_ entity-id] nil)
  (load-snapshot [_ entity-id] nil)

  Analytics
  (aggregate [_ query] nil)
  (time-series [_ query] nil))

;; Clients depend only on what they need
(defn query-service [reader]
  (query-events reader {:event-type "order.placed"}))

(defn ingestion-service [writer event]
  (save-event writer event))
```

---

## Dependency Inversion Principle (DIP)

> **High-level modules should not depend on low-level modules. Both should depend on abstractions.**

### What It Means

- Depend on interfaces/traits/protocols, not concrete implementations
- Abstractions should not depend on details; details should depend on abstractions

### Rust Examples

#### ❌ Violating DIP

```rust
// Bad: High-level module depends on low-level implementation
use parquet::file::writer::SerializedFileWriter;  // Concrete dependency!

pub struct EventService {
    storage: ParquetStorage,  // Depends on concrete type!
}

impl EventService {
    pub async fn ingest_event(&self, event: Event) -> Result<()> {
        self.storage.write_to_parquet(event).await?;  // Coupled to Parquet!
        Ok(())
    }
}

pub struct ParquetStorage {
    writer: SerializedFileWriter<File>,
}

impl ParquetStorage {
    pub async fn write_to_parquet(&self, event: Event) -> Result<()> {
        // Parquet-specific code
        Ok(())
    }
}
```

**Problems**:
- EventService is coupled to Parquet
- Can't swap to CSV, JSON, or other storage
- Hard to test (requires real Parquet files)

#### ✅ Following DIP

```rust
// Good: Both depend on abstraction (trait)

// Abstraction (defined in domain layer)
#[async_trait]
pub trait EventStorage: Send + Sync {
    async fn save(&self, event: Event) -> Result<(), StorageError>;
    async fn load(&self, id: Uuid) -> Result<Option<Event>, StorageError>;
}

// High-level module depends on abstraction
pub struct EventService {
    storage: Arc<dyn EventStorage>,  // Depends on trait!
}

impl EventService {
    pub fn new(storage: Arc<dyn EventStorage>) -> Self {
        Self { storage }
    }

    pub async fn ingest_event(&self, event: Event) -> Result<()> {
        self.storage.save(event).await?;  // Uses abstraction!
        Ok(())
    }
}

// Low-level module implements abstraction
pub struct ParquetStorage {
    writer: SerializedFileWriter<File>,
}

#[async_trait]
impl EventStorage for ParquetStorage {
    async fn save(&self, event: Event) -> Result<(), StorageError> {
        // Parquet implementation
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Event>, StorageError> {
        Ok(None)
    }
}

// Can easily add other implementations
pub struct CsvStorage { /* ... */ }

#[async_trait]
impl EventStorage for CsvStorage {
    async fn save(&self, event: Event) -> Result<(), StorageError> {
        // CSV implementation
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Event>, StorageError> {
        Ok(None)
    }
}

// Dependency injection
fn main() {
    let storage: Arc<dyn EventStorage> = Arc::new(ParquetStorage::new());
    // Or: let storage: Arc<dyn EventStorage> = Arc::new(CsvStorage::new());

    let service = EventService::new(storage);
    // Service works with any EventStorage implementation!
}
```

### Go Examples

#### ❌ Violating DIP

```go
// Bad: High-level depends on low-level
import "database/sql"

type UserService struct {
    db *sql.DB  // Concrete dependency on SQL!
}

func (s *UserService) CreateUser(username, email string) error {
    _, err := s.db.Exec(
        "INSERT INTO users (username, email) VALUES (?, ?)",
        username, email,
    )
    return err
}
```

#### ✅ Following DIP

```go
// Good: Both depend on abstraction

// Abstraction (interface)
type UserRepository interface {
    Save(ctx context.Context, user *User) error
    FindByID(ctx context.Context, id string) (*User, error)
}

// High-level module depends on abstraction
type UserService struct {
    repo UserRepository  // Depends on interface!
}

func NewUserService(repo UserRepository) *UserService {
    return &UserService{repo: repo}
}

func (s *UserService) CreateUser(ctx context.Context, username, email string) error {
    user := &User{
        ID:       uuid.New().String(),
        Username: username,
        Email:    email,
    }
    return s.repo.Save(ctx, user)  // Uses abstraction!
}

// Low-level module implements abstraction
type PostgresUserRepository struct {
    db *sql.DB
}

func NewPostgresUserRepository(db *sql.DB) *PostgresUserRepository {
    return &PostgresUserRepository{db: db}
}

func (r *PostgresUserRepository) Save(ctx context.Context, user *User) error {
    _, err := r.db.ExecContext(ctx,
        "INSERT INTO users (id, username, email) VALUES ($1, $2, $3)",
        user.ID, user.Username, user.Email,
    )
    return err
}

func (r *PostgresUserRepository) FindByID(ctx context.Context, id string) (*User, error) {
    // Implementation
    return nil, nil
}

// Can easily add other implementations
type MongoUserRepository struct {
    collection *mongo.Collection
}

func (r *MongoUserRepository) Save(ctx context.Context, user *User) error {
    _, err := r.collection.InsertOne(ctx, user)
    return err
}

func (r *MongoUserRepository) FindByID(ctx context.Context, id string) (*User, error) {
    return nil, nil
}

// Dependency injection
func main() {
    // Use Postgres
    db, _ := sql.Open("postgres", "...")
    repo := NewPostgresUserRepository(db)
    service := NewUserService(repo)

    // Or use Mongo
    // client, _ := mongo.Connect(...)
    // repo := NewMongoUserRepository(client.Database("mydb").Collection("users"))
    // service := NewUserService(repo)
}
```

### Clojure Examples

#### ❌ Violating DIP

```clojure
;; Bad: High-level depends on low-level
(ns myapp.user-service
  (:require [clojure.java.jdbc :as jdbc]))  ;; Concrete dependency!

(defn create-user
  [db username email]
  (jdbc/insert! db :users {:username username :email email}))  ;; Coupled to JDBC!
```

#### ✅ Following DIP

```clojure
;; Good: Both depend on abstraction

;; Abstraction (protocol)
(defprotocol UserRepository
  (save-user [this user])
  (find-user-by-id [this id]))

;; High-level module depends on abstraction
(defn create-user
  [repo username email]
  (let [user {:id (java.util.UUID/randomUUID)
              :username username
              :email email}]
    (save-user repo user)))  ;; Uses protocol!

;; Low-level module implements abstraction
(defrecord PostgresUserRepository [datasource]
  UserRepository
  (save-user [_ user]
    (jdbc/insert! datasource :users user))
  (find-user-by-id [_ id]
    (jdbc/query datasource ["SELECT * FROM users WHERE id = ?" id]
                {:result-set-fn first})))

;; Can easily add other implementations
(defrecord MongoUserRepository [db]
  UserRepository
  (save-user [_ user]
    (mongo/insert! db "users" user))
  (find-user-by-id [_ id]
    (mongo/find-one db "users" {:id id})))

;; Dependency injection
(defn -main []
  ;; Use Postgres
  (let [repo (->PostgresUserRepository (get-datasource))]
    (create-user repo "john" "john@example.com"))

  ;; Or use Mongo
  ;; (let [repo (->MongoUserRepository (get-mongo-db))]
  ;;   (create-user repo "john" "john@example.com"))
  )
```

---

## SOLID in Practice

### Applying All Five Principles Together

Let's see how all SOLID principles work together in a real-world example:

```rust
// Domain Layer (SRP: Each type has one responsibility)

// Entity
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub payload: Value,
}

// Value Object
pub struct EventType(String);

// Repository trait (DIP: Abstraction, ISP: Small interface)
pub trait EventRepository: Send + Sync {
    async fn save(&self, event: Event) -> Result<()>;
    async fn load(&self, id: Uuid) -> Result<Option<Event>>;
}

// Separate traits for different capabilities (ISP)
pub trait EventQuery: Send + Sync {
    async fn find_by_type(&self, event_type: &str) -> Result<Vec<Event>>;
}

// Application Layer (SRP: Use case has one responsibility)
pub struct IngestEventUseCase {
    repository: Arc<dyn EventRepository>,  // DIP: Depends on abstraction
    validator: Arc<dyn EventValidator>,    // DIP: Depends on abstraction
}

impl IngestEventUseCase {
    pub async fn execute(&self, request: IngestRequest) -> Result<IngestResponse> {
        // Validate
        self.validator.validate(&request)?;

        // Create domain entity
        let event = Event::new(request.event_type, request.payload)?;

        // Persist
        self.repository.save(event.clone()).await?;

        Ok(IngestResponse {
            event_id: event.id,
        })
    }
}

// Infrastructure Layer (OCP: Can add new implementations)

// Parquet implementation (LSP: Adheres to contract)
pub struct ParquetRepository { /* ... */ }

impl EventRepository for ParquetRepository {
    async fn save(&self, event: Event) -> Result<()> {
        // Parquet-specific implementation
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }
}

impl EventQuery for ParquetRepository {
    async fn find_by_type(&self, event_type: &str) -> Result<Vec<Event>> {
        Ok(vec![])
    }
}

// Can add CSV implementation without modifying existing code (OCP)
pub struct CsvRepository { /* ... */ }

impl EventRepository for CsvRepository {
    async fn save(&self, event: Event) -> Result<()> {
        Ok(())
    }

    async fn load(&self, id: Uuid) -> Result<Option<Event>> {
        Ok(None)
    }
}

impl EventQuery for CsvRepository {
    async fn find_by_type(&self, event_type: &str) -> Result<Vec<Event>> {
        Ok(vec![])
    }
}
```

---

## Anti-Patterns

### 1. God Object (Violates SRP)

```rust
// ❌ One struct does everything
pub struct AllSourceCore {
    // Too many responsibilities!
    events: Vec<Event>,
    tenants: Vec<Tenant>,
    users: Vec<User>,
    config: Config,
    metrics: Metrics,
}

impl AllSourceCore {
    pub fn save_event(&mut self, event: Event) { /* ... */ }
    pub fn query_events(&self, filter: Filter) -> Vec<Event> { /* ... */ }
    pub fn create_tenant(&mut self, tenant: Tenant) { /* ... */ }
    pub fn authenticate_user(&self, username: &str, password: &str) -> bool { /* ... */ }
    pub fn collect_metrics(&self) -> Metrics { /* ... */ }
    // ... 50 more methods!
}
```

### 2. Shotgun Surgery (Violates SRP/OCP)

```rust
// ❌ One change affects many files
// To add a new event type, must modify:
// - event.rs
// - serializer.rs
// - validator.rs
// - repository.rs
// - handler.rs
// - ... 10 more files!
```

**Solution**: Use polymorphism (traits) instead of conditionals

### 3. Leaky Abstraction (Violates DIP)

```rust
// ❌ Abstraction leaks implementation details
pub trait EventRepository {
    async fn save(&self, event: Event) -> Result<ParquetError>;  // Leaks Parquet!
    async fn execute_sql(&self, query: &str) -> Result<Vec<Event>>;  // Leaks SQL!
}
```

**Solution**: Use generic errors, hide implementation

---

## Refactoring Guide

### Step-by-Step Refactoring to SOLID

1. **Identify Responsibilities** (SRP)
   - List all the things a class does
   - Extract each responsibility into its own class/module

2. **Define Abstractions** (DIP)
   - Create traits/interfaces for dependencies
   - Make high-level code depend on abstractions

3. **Segregate Interfaces** (ISP)
   - Split large interfaces into smaller ones
   - Each client should only see methods it uses

4. **Enable Extension** (OCP)
   - Use traits/interfaces to allow new implementations
   - Avoid hard-coded type switches

5. **Verify Substitutability** (LSP)
   - Ensure subtypes can replace base types
   - Check that all implementations fulfill contracts

---

## Summary

| Principle | Key Question | Solution |
|-----------|-------------|----------|
| **SRP** | Does this have one reason to change? | Extract responsibilities into separate modules |
| **OCP** | Can I add features without modifying code? | Use traits/interfaces for extension |
| **LSP** | Can subtypes replace base types? | Ensure all implementations fulfill contracts |
| **ISP** | Do clients depend on unused methods? | Create small, focused interfaces |
| **DIP** | Do high-level modules depend on low-level? | Both depend on abstractions |

**Remember**:
- SOLID principles work together
- They support Clean Architecture
- Apply them gradually
- Don't over-engineer
- Test your abstractions

---

*This guide is part of the AllSource Event Store documentation. For questions or contributions, see [CONTRIBUTING.md](../CONTRIBUTING.md).*
