mod domain {
    use anyhow::Result;
    use async_trait::async_trait;
    use derive_new::new;

    #[derive(Debug, new)]
    pub struct User;

    #[derive(Debug, new)]
    pub struct Shop;

    #[derive(Debug, new)]
    pub struct Order;

    #[async_trait]
    pub trait UnitOfWork {
        fn create<T>(&mut self, aggregate: T) -> ()
        where
            T: Into<Aggregate>;

        fn update<T>(&mut self, aggregate: T) -> ()
        where
            T: Into<Aggregate>;

        fn delete<T>(&mut self, aggregate: T) -> ()
        where
            T: Into<Aggregate>;

        async fn commit(&mut self) -> Result<()>;
    }

    #[derive(Debug, new)]
    pub struct Command {
        pub aggregate: Aggregate,
        pub db_operation: DBOperation,
    }

    #[derive(Debug)]
    pub enum Aggregate {
        User(User),
        Shop(Shop),
        Order(Order),
    }

    #[derive(Debug)]
    pub enum DBOperation {
        Create,
        Update,
        Delete,
    }

    impl From<User> for Aggregate {
        fn from(user: User) -> Self {
            Self::User(user)
        }
    }

    impl From<Shop> for Aggregate {
        fn from(shop: Shop) -> Self {
            Self::Shop(shop)
        }
    }

    impl From<Order> for Aggregate {
        fn from(order: Order) -> Self {
            Self::Order(order)
        }
    }
}

mod infrastructure {
    use super::domain::{Aggregate, Command, DBOperation, Order, Shop, UnitOfWork, User};

    use anyhow::Context;
    use async_trait::async_trait;
    use derive_new::new;
    use sea_orm::{
        prelude::{DatabaseConnection, DbErr},
        DatabaseTransaction, TransactionTrait,
    };

    #[derive(new)]
    pub struct DatabaseClient {
        conn: DatabaseConnection,
        commands: Vec<Command>,
    }

    #[async_trait]
    impl UnitOfWork for DatabaseClient {
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
            self.conn
                .transaction::<_, (), DbErr>(|txn| {
                    Box::pin(async move {
                        for command in commands {
                            match command.aggregate {
                                Aggregate::User(user) => match command.db_operation {
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

    async fn create_user(_user: User, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn update_user(_user: User, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn delete_user(_user: User, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn create_shop(_shop: Shop, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn update_shop(_shop: Shop, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn delete_shop(_shop: Shop, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn create_order(_order: Order, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn update_order(_order: Order, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }

    async fn delete_order(_order: Order, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        todo!()
    }
}

mod context {
    use sea_orm::prelude::DatabaseConnection;

    use super::infrastructure::DatabaseClient;

    pub trait ProvideUnitOfWork {
        type UnitOfWork: super::domain::UnitOfWork + Send + Sync;
        fn provide(&self) -> Self::UnitOfWork;
    }

    pub struct Context {
        conn: DatabaseConnection,
    }

    impl ProvideUnitOfWork for Context {
        type UnitOfWork = super::infrastructure::DatabaseClient;

        fn provide(&self) -> Self::UnitOfWork {
            DatabaseClient::new(self.conn.clone(), vec![])
        }
    }
}

mod use_case {
    use super::{
        context::{Context, ProvideUnitOfWork},
        domain::{Order, Shop, UnitOfWork, User},
    };

    #[allow(unused)]
    async fn use_case(context: Context) -> anyhow::Result<()> {
        let mut uow = context.provide();

        uow.update(User::new());
        uow.update(Shop::new());
        uow.create(Order::new());
        uow.commit().await?;

        Ok(())
    }
}
