use std::marker::PhantomData;

use bevy_ecs::prelude::*;

use crate::{Observable, ReactiveContext, Signal, SignalData};

/// A derived component whose value is computed automatically, and can only be read through the
/// [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Derived<T: Send + Sync + 'static> {
    pub(crate) reactor_entity: Entity,
    pub(crate) p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq + 'static> Observable<T> for Derived<T> {
    fn reactor_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync + 'static> Clone for Derived<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + 'static> Copy for Derived<T> {}

impl<T: Send + Sync + 'static> Derived<T> {
    pub fn read<'r>(&self, rctx: &'r mut ReactiveContext) -> &'r T {
        rctx.world
            .get::<SignalData<T>>(self.reactor_entity)
            .unwrap()
            .data()
    }
}

/// Lives alongside the observable component. This holds a system that can be called without the
/// caller knowing any type information, and will update the associated observable component.
#[derive(Component)]
pub(crate) struct DerivedData {
    function: Box<dyn DeriveFn>,
}

pub(crate) trait ObservableData: Send + Sync + PartialEq + 'static {}
impl<T: Send + Sync + PartialEq + 'static> ObservableData for T {}

trait DeriveFn: Send + Sync + 'static + FnMut(&mut World) {}
impl<T: Send + Sync + 'static + FnMut(&mut World)> DeriveFn for T {}

impl DerivedData {
    pub(crate) fn new<C: Send + Sync + PartialEq + 'static, D: Derivable<C> + 'static>(
        derived: Entity,
        input_deps: D,
        derive_fn: fn(D::Query<'_>) -> C,
    ) -> Self {
        let function = move |world: &mut World| {
            let derived_result = D::read_and_derive(world, derived, derive_fn, input_deps);
            // using write here will trigger all subscribers, which in turn can trigger this
            // function through derived functions, traversing the entire graph.
            SignalData::send_signal(world, derived, derived_result);
        };
        let function = Box::new(function);
        Self { function }
    }

    pub(crate) fn execute(&mut self, world: &mut World) {
        (self.function)(world);
    }
}

/// A type that represents a collection of observables, which can be used to compute a derived
/// value.
pub trait Derivable<T>: Copy + Send + Sync + 'static {
    type Query<'a>;
    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        input_deps: Self,
    ) -> T;
}

impl<A: ObservableData, T> Derivable<T> for Signal<A> {
    type Query<'a> = &'a A;

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [e.reactor_entity];
        let [mut a] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);

        derive_fn(a.get::<SignalData<A>>().unwrap().data())
    }
}

impl<A: ObservableData, B: ObservableData, T> Derivable<T> for (Signal<A>, Signal<B>) {
    type Query<'a> = (&'a A, &'a B);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [e.0.reactor_entity, e.1.reactor_entity];
        let [mut a, mut b] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
        ))
    }
}

impl<A: ObservableData, B: ObservableData, C: ObservableData, T> Derivable<T>
    for (Signal<A>, Signal<B>, Signal<C>)
{
    type Query<'a> = (&'a A, &'a B, &'a C);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [e.0.reactor_entity, e.1.reactor_entity, e.2.reactor_entity];
        let [mut a, mut b, mut c] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);
        c.get_mut::<SignalData<C>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
            c.get::<SignalData<C>>().unwrap().data(),
        ))
    }
}

impl<A: ObservableData, B: ObservableData, C: ObservableData, D: ObservableData, T> Derivable<T>
    for (Signal<A>, Signal<B>, Signal<C>, Signal<D>)
{
    type Query<'a> = (&'a A, &'a B, &'a C, &'a D);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [
            e.0.reactor_entity,
            e.1.reactor_entity,
            e.2.reactor_entity,
            e.3.reactor_entity,
        ];
        let [mut a, mut b, mut c, mut d] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);
        c.get_mut::<SignalData<C>>().unwrap().add_subscriber(reader);
        d.get_mut::<SignalData<D>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
            c.get::<SignalData<C>>().unwrap().data(),
            d.get::<SignalData<D>>().unwrap().data(),
        ))
    }
}

impl<
        A: ObservableData,
        B: ObservableData,
        C: ObservableData,
        D: ObservableData,
        E: ObservableData,
        T,
    > Derivable<T> for (Signal<A>, Signal<B>, Signal<C>, Signal<D>, Signal<E>)
{
    type Query<'a> = (&'a A, &'a B, &'a C, &'a D, &'a E);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [
            e.0.reactor_entity,
            e.1.reactor_entity,
            e.2.reactor_entity,
            e.3.reactor_entity,
            e.4.reactor_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);
        c.get_mut::<SignalData<C>>().unwrap().add_subscriber(reader);
        d.get_mut::<SignalData<D>>().unwrap().add_subscriber(reader);
        e.get_mut::<SignalData<E>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
            c.get::<SignalData<C>>().unwrap().data(),
            d.get::<SignalData<D>>().unwrap().data(),
            e.get::<SignalData<E>>().unwrap().data(),
        ))
    }
}

impl<
        A: ObservableData,
        B: ObservableData,
        C: ObservableData,
        D: ObservableData,
        E: ObservableData,
        F: ObservableData,
        T,
    > Derivable<T>
    for (
        Signal<A>,
        Signal<B>,
        Signal<C>,
        Signal<D>,
        Signal<E>,
        Signal<F>,
    )
{
    type Query<'a> = (&'a A, &'a B, &'a C, &'a D, &'a E, &'a F);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [
            e.0.reactor_entity,
            e.1.reactor_entity,
            e.2.reactor_entity,
            e.3.reactor_entity,
            e.4.reactor_entity,
            e.5.reactor_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e, mut f] =
            world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);
        c.get_mut::<SignalData<C>>().unwrap().add_subscriber(reader);
        d.get_mut::<SignalData<D>>().unwrap().add_subscriber(reader);
        e.get_mut::<SignalData<E>>().unwrap().add_subscriber(reader);
        f.get_mut::<SignalData<F>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
            c.get::<SignalData<C>>().unwrap().data(),
            d.get::<SignalData<D>>().unwrap().data(),
            e.get::<SignalData<E>>().unwrap().data(),
            f.get::<SignalData<F>>().unwrap().data(),
        ))
    }
}

impl<
        A: ObservableData,
        B: ObservableData,
        C: ObservableData,
        D: ObservableData,
        E: ObservableData,
        F: ObservableData,
        G: ObservableData,
        T,
    > Derivable<T>
    for (
        Signal<A>,
        Signal<B>,
        Signal<C>,
        Signal<D>,
        Signal<E>,
        Signal<F>,
        Signal<G>,
    )
{
    type Query<'a> = (&'a A, &'a B, &'a C, &'a D, &'a E, &'a F, &'a G);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [
            e.0.reactor_entity,
            e.1.reactor_entity,
            e.2.reactor_entity,
            e.3.reactor_entity,
            e.4.reactor_entity,
            e.5.reactor_entity,
            e.6.reactor_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g] =
            world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);
        c.get_mut::<SignalData<C>>().unwrap().add_subscriber(reader);
        d.get_mut::<SignalData<D>>().unwrap().add_subscriber(reader);
        e.get_mut::<SignalData<E>>().unwrap().add_subscriber(reader);
        f.get_mut::<SignalData<F>>().unwrap().add_subscriber(reader);
        g.get_mut::<SignalData<G>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
            c.get::<SignalData<C>>().unwrap().data(),
            d.get::<SignalData<D>>().unwrap().data(),
            e.get::<SignalData<E>>().unwrap().data(),
            f.get::<SignalData<F>>().unwrap().data(),
            g.get::<SignalData<G>>().unwrap().data(),
        ))
    }
}

impl<
        A: ObservableData,
        B: ObservableData,
        C: ObservableData,
        D: ObservableData,
        E: ObservableData,
        F: ObservableData,
        G: ObservableData,
        H: ObservableData,
        T,
    > Derivable<T>
    for (
        Signal<A>,
        Signal<B>,
        Signal<C>,
        Signal<D>,
        Signal<E>,
        Signal<F>,
        Signal<G>,
        Signal<H>,
    )
{
    type Query<'a> = (&'a A, &'a B, &'a C, &'a D, &'a E, &'a F, &'a G, &'a H);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [
            e.0.reactor_entity,
            e.1.reactor_entity,
            e.2.reactor_entity,
            e.3.reactor_entity,
            e.4.reactor_entity,
            e.5.reactor_entity,
            e.6.reactor_entity,
            e.7.reactor_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] =
            world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<SignalData<A>>().unwrap().add_subscriber(reader);
        b.get_mut::<SignalData<B>>().unwrap().add_subscriber(reader);
        c.get_mut::<SignalData<C>>().unwrap().add_subscriber(reader);
        d.get_mut::<SignalData<D>>().unwrap().add_subscriber(reader);
        e.get_mut::<SignalData<E>>().unwrap().add_subscriber(reader);
        f.get_mut::<SignalData<F>>().unwrap().add_subscriber(reader);
        g.get_mut::<SignalData<G>>().unwrap().add_subscriber(reader);
        h.get_mut::<SignalData<H>>().unwrap().add_subscriber(reader);

        derive_fn((
            a.get::<SignalData<A>>().unwrap().data(),
            b.get::<SignalData<B>>().unwrap().data(),
            c.get::<SignalData<C>>().unwrap().data(),
            d.get::<SignalData<D>>().unwrap().data(),
            e.get::<SignalData<E>>().unwrap().data(),
            f.get::<SignalData<F>>().unwrap().data(),
            g.get::<SignalData<G>>().unwrap().data(),
            h.get::<SignalData<H>>().unwrap().data(),
        ))
    }
}
