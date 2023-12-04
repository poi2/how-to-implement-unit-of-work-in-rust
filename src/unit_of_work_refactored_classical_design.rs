mod domain {
    use anyhow::Result;
    use async_trait::async_trait;
    use derive_new::new;

    #[derive(Debug, new)]
    pub struct User;

    #[derive(Debug, new)]
    pub struct Shop;

    #[async_trait]
    pub trait UnitOfWork {
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

    pub trait UserRepository {
        fn create_user(&mut self, user: User) -> Result<()>;
        fn update_user(&mut self, user: User) -> Result<()>;
        fn delete_user(&mut self, user: User) -> Result<()>;
    }

    pub trait ShopRepository {
        fn create_shop(&mut self, shop: Shop) -> Result<()>;
        fn update_shop(&mut self, shop: Shop) -> Result<()>;
        fn delete_shop(&mut self, shop: Shop) -> Result<()>;
    }
}

mod infrastructure {
    use super::domain::{
        Aggregate, Command, DBOperation, Shop, ShopRepository, UnitOfWork, User, UserRepository,
    };

    use anyhow::{Context, Result};
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
        async fn commit(&mut self) -> Result<()> {
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
        unimplemented!()
    }

    async fn update_user(_user: User, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        unimplemented!()
    }

    async fn delete_user(_user: User, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        unimplemented!()
    }

    async fn create_shop(_shop: Shop, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        unimplemented!()
    }

    async fn update_shop(_shop: Shop, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        unimplemented!()
    }

    async fn delete_shop(_shop: Shop, _txn: &DatabaseTransaction) -> Result<(), DbErr> {
        unimplemented!()
    }

    impl UserRepository for DatabaseClient {
        fn create_user(&mut self, user: User) -> Result<()> {
            self.commands
                .push(Command::new(user.into(), DBOperation::Create));
            Ok(())
        }

        fn update_user(&mut self, user: User) -> Result<()> {
            self.commands
                .push(Command::new(user.into(), DBOperation::Update));
            Ok(())
        }

        fn delete_user(&mut self, user: User) -> Result<()> {
            self.commands
                .push(Command::new(user.into(), DBOperation::Delete));
            Ok(())
        }
    }

    impl ShopRepository for DatabaseClient {
        fn create_shop(&mut self, shop: Shop) -> Result<()> {
            self.commands
                .push(Command::new(shop.into(), DBOperation::Create));
            Ok(())
        }

        fn update_shop(&mut self, shop: Shop) -> Result<()> {
            self.commands
                .push(Command::new(shop.into(), DBOperation::Update));
            Ok(())
        }

        fn delete_shop(&mut self, shop: Shop) -> Result<()> {
            self.commands
                .push(Command::new(shop.into(), DBOperation::Delete));
            Ok(())
        }
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
        domain::{Shop, ShopRepository, UnitOfWork, User, UserRepository},
    };

    #[allow(unused)]
    async fn use_case(context: Context) -> anyhow::Result<()> {
        let mut uow = context.provide();

        uow.update_user(User::new());
        uow.update_shop(Shop::new());
        uow.commit().await?;

        Ok(())
    }
}
