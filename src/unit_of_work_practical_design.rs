mod domain {
    use anyhow::Result;
    use async_trait::async_trait;
    use derive_new::new;

    #[derive(Debug, new)]
    pub struct User;

    impl User {
        pub fn is_valid(&self) -> bool {
            true
        }
    }

    #[derive(Debug, new)]
    pub struct Shop;

    #[derive(Debug, new)]
    pub struct Order;

    #[async_trait]
    pub trait UnitOfWork {
        async fn begin(&mut self) -> Result<()>;
        async fn commit(&mut self) -> Result<()>;
        async fn rollback(&mut self) -> Result<()>;
    }

    #[async_trait]
    pub trait UserRepository {
        async fn create_user(&self, user: User) -> Result<User>;
        async fn update_user(&self, user: User) -> Result<User>;
        async fn delete_user(&self, user: User) -> Result<()>;
    }

    #[async_trait]
    pub trait ShopRepository {
        async fn create_shop(&self, shop: Shop) -> Result<Shop>;
        async fn update_shop(&self, shop: Shop) -> Result<Shop>;
        async fn delete_shop(&self, shop: Shop) -> Result<()>;
    }

    #[async_trait]
    pub trait OrderRepository {
        async fn create_order(&self, order: Order) -> Result<Order>;
        async fn update_order(&self, order: Order) -> Result<Order>;
        async fn delete_order(&self, order: Order) -> Result<()>;
    }
}

mod infrastructure {
    use super::domain::{
        Order, OrderRepository, Shop, ShopRepository, UnitOfWork, User, UserRepository,
    };

    use anyhow::{bail, Context};
    use async_trait::async_trait;
    use derive_new::new;
    use sea_orm::{prelude::DatabaseConnection, DatabaseTransaction, TransactionTrait};

    #[derive(new)]
    pub struct UnitOfWorkImpl {
        conn: DatabaseConnection,
        txn: Option<DatabaseTransaction>,
    }

    #[async_trait]
    impl UnitOfWork for UnitOfWorkImpl {
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

    #[async_trait]
    impl UserRepository for UnitOfWorkImpl {
        async fn create_user(&self, _user: User) -> anyhow::Result<User> {
            unimplemented!()
        }

        async fn update_user(&self, _user: User) -> anyhow::Result<User> {
            unimplemented!()
        }

        async fn delete_user(&self, _user: User) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl ShopRepository for UnitOfWorkImpl {
        async fn create_shop(&self, _shop: Shop) -> anyhow::Result<Shop> {
            unimplemented!()
        }

        async fn update_shop(&self, _shop: Shop) -> anyhow::Result<Shop> {
            unimplemented!()
        }

        async fn delete_shop(&self, _shop: Shop) -> anyhow::Result<()> {
            unimplemented!()
        }
    }

    #[async_trait]
    impl OrderRepository for UnitOfWorkImpl {
        async fn create_order(&self, _order: Order) -> anyhow::Result<Order> {
            unimplemented!()
        }

        async fn update_order(&self, _order: Order) -> anyhow::Result<Order> {
            unimplemented!()
        }

        async fn delete_order(&self, _order: Order) -> anyhow::Result<()> {
            unimplemented!()
        }
    }
}

mod context {
    use sea_orm::DatabaseConnection;

    use super::infrastructure::UnitOfWorkImpl;

    pub trait ProvideUnitOfWork {
        type UnitOfWork: super::domain::UnitOfWork + Send + Sync;
        fn provide(&self) -> Self::UnitOfWork;
    }

    pub struct Context {
        conn: DatabaseConnection,
    }

    impl ProvideUnitOfWork for Context {
        type UnitOfWork = super::infrastructure::UnitOfWorkImpl;

        fn provide(&self) -> Self::UnitOfWork {
            UnitOfWorkImpl::new(self.conn.clone(), None)
        }
    }
}

mod use_case {
    use super::{
        context::{Context, ProvideUnitOfWork},
        domain::{Order, OrderRepository, Shop, ShopRepository, UnitOfWork, User, UserRepository},
    };

    #[allow(unused)]
    async fn use_case(context: Context) -> anyhow::Result<()> {
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

        Ok(())
    }
}
