//! Repository layer. Thin wrappers around SeaORM entity calls — keeps tests focused
//! on a single trait surface when we mock the DB. Currently empty: handlers go
//! straight through `Entity::find` until a clear repeated pattern emerges.
