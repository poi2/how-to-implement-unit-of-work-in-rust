Rust における Unit of Work の実装例

# はじめに

データベースを使ったアプリケーションを実装する際、集約をまたいだトランザクション処理が必要になるケースがあります。  
リッチなデータベースのライブラリーが提供されている言語においては、ライブラリーがそのような処理に対応しているため、導入が容易です。  
残念なことに Rust においてはそのような機能はライブラリーには提供されていないため、開発者は自分で実装する必要があります。  
それを行うには Rust 固有の所有権やライフタイムの概念を理解していないと実装が難しいこと、また英語、日本語問わず Rust におけるそれらの実装例が少ないため、実装に苦労することが多いです。  
今回は Rust で Clean Architecture でアプリケーションを構成した状態で、集約をまたいだトランザクション処理を実装例を２つ紹介し、それぞれの実装方法のメリットとデメリットも紹介します。

# Clean Architecture における集約をまたいだトランザクション処理の実装方針

具体例があるほうが説明がしやすいため、「ユーザーがショップで商品を購入する場合、User, Shop, Order 集約を更新・作成する」を例に説明します。  

Clean Architecture でアプリケーションを構成した場合、トランザクション処理は Repository に実装されます。  
Unit of Work パターンを使わない場合、以下のようなコードです。

```rust
user_repository.update(user).await?;
shop_repository.update(shop).await?;
order_repository.create(order).await?;
```

通常 Clean Architecture の Repository は集約ごとに実装されることが多く、その場合、上記のコードでは User, Shop, Order をそれぞれ個別のトランザクションで更新することになります。  
その場合、途中で失敗した場合にデータ不整合を発生させる危険性が生じます。  
手動でロールバック処理を実行すればよいですが、それはとても考慮すべき事項が多く大変ですし、同一データベースに対して保存されるデータであるなら、筋は悪いです。  

マーティン・ファウラーのエンタープライズアプリケーションアーキテクチャパターン(*1, *2, 以降 PoEAA) では、このような場合に Unit of Work パターンでトランザクション処理の実現する方法が紹介されています。  
詳細は参考文献を読んでいただくのが一番ですが、具体的には複数の集約や異なる集約に対しての永続化（新規作成、更新、削除）をトランザクション処理するものです。  
本記事では実装例を２つ紹介し、それぞれのメリットとデメリットを紹介します。

# トランザクション処理を必要とする要件

まず最初に想定する要件について整理しましょう。  
本記事では以下のような要件を考慮して設計・実装を行います。  

- [MUST] 複数の異なる集約を一括して永続化できる
- [MUST] 集約を永続化する順番を指定できる
    - 外部キー制約がある場合、正しい順序で INSERT や DELETE を実行する必要があるため
- [SHOULD] 更新後のデータベースの状態をトランザクション内で取得できる
    - 更新後の状態をもとに集約を再生成する要件があったときに必要になる
    - データベースが採番する id を利用して外部キー制約を満たすテーブルへ INSERT する要件があったときに必要になる
    - 更新後のデータベースの状態をチェックしロールバックする要件があったときに必要になる

# 実装例

実装例ではそれらの要件をなるべく満たすことを目標にしますが、実際にすべての要件を満たすことができるとは限りません。  
MUST は必ず必要ですが、SHOULD は要件次第で必要になる場合があります。  
この後説明する実装例１では MUST を、実装例２では MUST と SHOULD を満たすように実装します。

なお、今回はデータベースクライアントとして SeaORM を利用しています。
Diesel を仕様したとしても同様の方法で実装が可能です。

記事では Unit of Work の実装にフォーカスできるよう、一部のコードを省略して書いています。
もし全体のコードを見たい場合は Github のリポジトリーを参照してください。

## 実装例１：更新対象の集約をストックし、最後にまとめて更新する方法

更新対象の集約をストックし、最後にまとめて更新する方法を実装例１で紹介します。  
PoEAA で紹介されている Unit of Work に近い実装方法がこれにあたります。  
例えば UnitOfWork trait と DatabaseClient が以下のようになっているとします。

```rust
trait UnitOfWork {
    // 新規作成する集約をストックする
    fn create<T>(&mut self, aggregate: T) -> ()
    where
        T: Into<Aggregate>;

    // 更新する集約をストックする
    fn update<T>(&mut self, aggregate: T) -> ()
    where
        T: Into<Aggregate>;

    // 削除する集約をストックする
    fn delete<T>(&mut self, aggregate: T) -> ()
    where
        T: Into<Aggregate>;

    // ストックした集約をまとめて更新する
    async fn commit(&mut self) -> Result<()>;
}

// UnitOfWork を実装する構造体
// データベースコネクションと、更新用のコマンドをストックする
struct DatabaseClient {
    conn: DatabaseConnection,
    commands: Vec<Command>,
}

// 更新対象とデータベース操作をまとめた構造体
struct Command {
    aggregate: Aggregate,
    db_operation: DBOperation,
}

// 更新対象となる集約を表現する型
enum Aggregate {
    User(User),
    Shop(Shop),
    Order(Order),
}

// データベース操作の種別を表現する型
enum DBOperation {
    Create,
    Update,
    Delete,
}

// 各集約を Aggregate へ変換する From trait の実装
impl From<User> for Aggregate {
    fn from(user: User) -> Self {
        Self::User(user)
    }
}
// Shop, Order も同様に実装する
```

DatabaseClient に UnitOfWork を実装しましょう。

```rust
impl UnitOfWork for DatabaseClient {
    // 新規作成する集約をストックする
    // T は Aggregate へ変換可能な型であればなんでもよい
    // 今後、新しい集約を追加したい場合、集約から Aggregate への変換を実装するだけで対応できる
    fn create<T>(&mut self, aggregate: T) -> ()
    where
        T: Into<Aggregate>,
    {
        self.commands
            .push(Command::new(aggregate.into(), DBOperation::Create));
    }

    fn update<T>(&mut self, aggregate: T) -> ()
    where
        T: Into<Aggregate>,
    {
        self.commands
            .push(Command::new(aggregate.into(), DBOperation::Update));
    }

    fn delete<T>(&mut self, aggregate: T) -> ()
    where
        T: Into<Aggregate>,
    {
        self.commands
            .push(Command::new(aggregate.into(), DBOperation::Delete));
    }

    async fn commit(&mut self) -> anyhow::Result<()> {
        let commands = self.commands.drain(..).collect::<Vec<_>>();
        // トランザクションを開始して、トランザクションの中でクエリーを実行する
        self.conn
            .transaction::<_, (), DbErr>(|txn| {
                Box::pin(async move {
                    for command in commands {
                        // コマンドに対応するクエリーを実行する
                        match command.aggregate {
                            Aggregate::User(user) => match command.db_operation {
                                // 本記事では SeaORM の書き方は本題ではないため省略しますが、
                                // 全体のコードを見たい場合は Github を参照してください。
                                DBOperation::Create => create_user(user, txn).await,
                                DBOperation::Update => update_user(user, txn).await,
                                DBOperation::Delete => delete_user(user, txn).await,
                            },
                            Aggregate::Shop(shop) => match command.db_operation {
                                DBOperation::Create => create_shop(shop, txn).await,
                                DBOperation::Update => update_shop(shop, txn).await,
                                DBOperation::Delete => delete_shop(shop, txn).await,
                            },
                            Aggregate::Order(order) => match command.db_operation {
                                DBOperation::Create => create_order(order, txn).await,
                                DBOperation::Update => update_order(order, txn).await,
                                DBOperation::Delete => delete_order(order, txn).await,
                            },
                        }?;
                    }

                    Ok(())
                })
            })
            .await
            .with_context(|| format!("failed to commit transaction"))?;
        Ok(())
    }
}
```

この DatabaseClient を Use case で使う場合、以下のようなコードで記述できます。  
Use case で UnitOfWork の実態を取得する方法はいくつかありますが、本記事では DI コンテナーを使う使ったサンプルコードを紹介します。

```rust
// Context から UnitOfWork を取得する
let mut uow = context.provide();
// User, Shop を更新、Order を新規作成する
uow.update(user);
uow.update(shop);
uow.commit().await?;
```

### Pros/Cons

- Pros
    - MUST の要件はすべてクリアしている
- Cons
    - 対応する集約を追加・削除した場合、DatabaseClient 本体のコードを修正する必要がある
    - UnitOfWork trait が定義する create, update, delete のメソッドに依存するため、それ以外の新しい操作を追加する要件がでてきたとき複数箇所のコード修正が必要になる
        - （Github で公開しているサンプルコードにはリファクタリングした実装例があります）
    - SHOULD で上げた「更新後のデータベースの状態をトランザクション内で取得する」には対応が難しい

Cons が問題にならないアプリケーションにおいては、この実装方法で十分です。

## 実装例２：トランザクションを開始後、トランザクション内でクエリーを都度実行し、最後にコミットする方法

SHOULD で上げた「更新後のデータベースの状態をトランザクション内で取得する」に対応する実装方法を考えます。  
実装例１では、実際にコミットが実行されるまで、データベースに対するクエリーは実行されません。  
そのため、更新後のデータベースの状態をトランザクション内で取得することができませんでした。  

では、トランザクションを開始後、トランザクション内でクエリーを都度実行し、最後にコミットする方法はどうでしょうか。  
そうすれば、更新後のデータベースの状態をトランザクション内で取得することができます。  
この方針は PoEAA で紹介されている Unit of Work とは若干異なりますが、ほぼ同じ効果が得られます。  

また、実装例１では対応する集約を追加・削除したときの修正が DatabaseClient に及ぶことが気になるので、Open-Closed Principle を満たすように設計・実装します。  

この実装例では UnitOfWork trait は以下のようになります。  
トランザクションの管理のみ扱う trait として定義します。  

```rust
trait UnitOfWork {
    async fn begin(&mut self) -> Result<()>;
    async fn commit(&mut self) -> Result<()>;
    async fn rollback(&mut self) -> Result<()>;
}
```

そして、User, Shop, Order の CRUD 処理を行うための trait　を UserRepository, ShopRepository, OrderRepository とし、以下のように定義します。  
現在 Repository たちは同じメソッドを持っていますが、要件ごとに異なるメソッドを持たせることも可能です。  
こうすることで、別の集約を操作する Repository を追加した場合、他の Repository や DatabaseClient に影響を与えることなく実装を追加できます。  

```rust
trait UserRepository {
    async fn create_user(&self, user: User) -> Result<User>;
    async fn update_user(&self, user: User) -> Result<User>;
    async fn delete_user(&self, user: User) -> Result<()>;
}

trait ShopRepository {
    async fn create_shop(&self, shop: Shop) -> Result<Shop>;
    async fn update_shop(&self, shop: Shop) -> Result<Shop>;
    async fn delete_shop(&self, shop: Shop) -> Result<()>;
}

trait OrderRepository {
    async fn create_order(&self, order: Order) -> Result<Order>;
    async fn update_order(&self, order: Order) -> Result<Order>;
    async fn delete_order(&self, order: Order) -> Result<()>;
}
```

続いて実装です。

```rust
// UnitOfWork を実装する構造体
// データベースコネクションとトランザクションを保持する
struct DatabaseClient {
    conn: DatabaseConnection,
    txn: Option<DatabaseTransaction>,
}

impl UnitOfWork for DatabaseClient {
    // トランザクションの開始
    async fn begin(&mut self) -> anyhow::Result<()> {
        if self.txn.is_none() {
            let txn = self
                .conn
                .begin()
                .await
                .with_context(|| "Failed to begin transaction")?;
            self.txn = Some(txn);
            Ok(())
        } else {
            bail!("Transaction is already started")
        }
    }

    // トランザクションのコミット
    async fn commit(&mut self) -> anyhow::Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.commit()
                .await
                .with_context(|| "Failed to commit transaction")?;
            Ok(())
        } else {
            bail!("Transaction is not started")
        }
    }

    // トランザクションのロールバック
    async fn rollback(&mut self) -> anyhow::Result<()> {
        if let Some(txn) = self.txn.take() {
            txn.rollback()
                .await
                .with_context(|| "Failed to rollback transaction")?;
            Ok(())
        } else {
            bail!("Transaction is not started")
        }
    }
}
```

こうした場合 Use case では以下のように利用できます。
commit する前にトランザクション内で取得した user をもとにコミットかロールバックを決定することができます。

```rust
let mut uow = context.provide();

uow.begin().await?;
let user = uow.update_user(User::new()).await?;
let shop = uow.update_shop(Shop::new()).await?;
let order = uow.update_order(Order::new()).await?;
if user.is_valid() {
    uow.commit().await?;
} else {
    uow.rollback().await?;
}
```

### Pros/Cons

- pros
    - MUST, SHOULD のすべてに対応している
    - 対応する集約を追加・削除しても DatabaseClient に新しい Repository trait を impl するだけで対応できる
- cons
    - Unit of Work を使わない単純な Repository も欲しい場合は Unit of Work と Repository 用の２つの実装を用意する必要がある
    - 使い方によってはトランザクションの持続時間が長くなり、デッドロックのリスクがあがる

当初考慮した要件をすべて満たすことができました。

# おわりに

今回は Clean Architecture でアプリケーションを構成した状態で、集約をまたいだトランザクション処理を行う実装例を２つ紹介しました。  
実装例１は PoEAA で紹介されている Unit of Work に近い手法を採用し、これはイージーに実装できます。  
実装例２はより実践的な要件に対応し、コードもシンプルになりましたが、少しテクニカルなコードかもしれません。  
要件に応じて実装方法を選択してください。  

Unit of Work は常に使うべきかというわけでもありません。  
要件によっては Repository を使うだけのほうがよい場合があります。  
本当に必要になったときに使ってください。  

# 参考文献

- *1: マーティン・ファウラー. [エンタープライズアプリケーションアーキテクチャパターン](https://www.shoeisha.co.jp/book/detail/9784798105536). 2005
- *2: Martin Fowler. [Unit of Work](https://martinfowler.com/eaaCatalog/unitOfWork.html).
- *3: Daisuke ito. [How to implement Unit of Work in Rust]()
