use bevy_ecs::prelude::*;

use crate::{Obs, Observable};

/// Lives alongside the observable component. This holds a system that can be called without the
/// caller knowing any type information, and will update the associated observable component.
#[derive(Component)]
pub(crate) struct Derived {
    function: Box<dyn DeriveFn>,
}

pub(crate) trait ObservableData: Send + Sync + PartialEq + 'static {}
impl<T: Send + Sync + PartialEq + 'static> ObservableData for T {}

trait DeriveFn: Send + Sync + 'static + FnMut(&mut World) {}
impl<T: Send + Sync + 'static + FnMut(&mut World)> DeriveFn for T {}

impl Derived {
    pub(crate) fn new<C: Send + Sync + PartialEq + 'static, D: Derivable<C> + 'static>(
        derived: Entity,
        input_deps: D,
        derive_fn: fn(D::Query<'_>) -> C,
    ) -> Self {
        let function = move |world: &mut World| {
            let derived_result = D::read_and_derive(world, derived, derive_fn, input_deps);
            // using write here will trigger all subscribers, which in turn can trigger this
            // function through derived functions, traversing the entire graph.
            write_observable::<C>(world, derived, derived_result);
        };
        let function = Box::new(function);
        Self { function }
    }

    pub(crate) fn execute(&mut self, world: &mut World) {
        (self.function)(world);
    }
}

/// Write observable data to the supplied entity. The [`Observable`] component will be added if it
/// is missing.
pub fn write_observable<T: Send + Sync + PartialEq + 'static>(
    world: &mut World,
    entity: Entity,
    value: T,
) {
    // the reader must have a way to update itself when the subscription is invoked
    // the obs may be a button, but the reader may be a door, need to get around type issues?
    // maybe the type erased thing being invoked is a one shot system
    // can we stick the closure inside a system?

    if let Some(mut obs) = world.get_mut::<Observable<T>>(entity) {
        if obs.data == value {
            return;
        }
        obs.data = value;
        let mut subscribers = std::mem::take(&mut obs.subscribers);
        for sub in subscribers.drain() {
            let mut derived = world.entity_mut(sub).take::<Derived>().unwrap();
            (derived.function)(world);
            world.entity_mut(sub).insert(derived);
        }
    } else {
        world.entity_mut(entity).insert(Observable {
            data: value,
            subscribers: Default::default(),
        });
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

impl<A: ObservableData, T> Derivable<T> for Obs<A> {
    type Query<'a> = &'a A;

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [e.rctx_entity];
        let [mut a] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn(&a.get::<Observable<A>>().unwrap().data)
    }
}

impl<A: ObservableData, B: ObservableData, T> Derivable<T> for (Obs<A>, Obs<B>) {
    type Query<'a> = (&'a A, &'a B);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [e.0.rctx_entity, e.1.rctx_entity];
        let [mut a, mut b] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
        ))
    }
}

impl<A: ObservableData, B: ObservableData, C: ObservableData, T> Derivable<T>
    for (Obs<A>, Obs<B>, Obs<C>)
{
    type Query<'a> = (&'a A, &'a B, &'a C);

    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: fn(Self::Query<'_>) -> T,
        entities: Self,
    ) -> T {
        let e = entities;
        let entities = [e.0.rctx_entity, e.1.rctx_entity, e.2.rctx_entity];
        let [mut a, mut b, mut c] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);
        c.get_mut::<Observable<C>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
            &c.get::<Observable<C>>().unwrap().data,
        ))
    }
}

impl<A: ObservableData, B: ObservableData, C: ObservableData, D: ObservableData, T> Derivable<T>
    for (Obs<A>, Obs<B>, Obs<C>, Obs<D>)
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
            e.0.rctx_entity,
            e.1.rctx_entity,
            e.2.rctx_entity,
            e.3.rctx_entity,
        ];
        let [mut a, mut b, mut c, mut d] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);
        c.get_mut::<Observable<C>>()
            .unwrap()
            .subscribers
            .insert(reader);
        d.get_mut::<Observable<D>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
            &c.get::<Observable<C>>().unwrap().data,
            &d.get::<Observable<D>>().unwrap().data,
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
    > Derivable<T> for (Obs<A>, Obs<B>, Obs<C>, Obs<D>, Obs<E>)
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
            e.0.rctx_entity,
            e.1.rctx_entity,
            e.2.rctx_entity,
            e.3.rctx_entity,
            e.4.rctx_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e] = world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);
        c.get_mut::<Observable<C>>()
            .unwrap()
            .subscribers
            .insert(reader);
        d.get_mut::<Observable<D>>()
            .unwrap()
            .subscribers
            .insert(reader);
        e.get_mut::<Observable<E>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
            &c.get::<Observable<C>>().unwrap().data,
            &d.get::<Observable<D>>().unwrap().data,
            &e.get::<Observable<E>>().unwrap().data,
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
    > Derivable<T> for (Obs<A>, Obs<B>, Obs<C>, Obs<D>, Obs<E>, Obs<F>)
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
            e.0.rctx_entity,
            e.1.rctx_entity,
            e.2.rctx_entity,
            e.3.rctx_entity,
            e.4.rctx_entity,
            e.5.rctx_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e, mut f] =
            world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);
        c.get_mut::<Observable<C>>()
            .unwrap()
            .subscribers
            .insert(reader);
        d.get_mut::<Observable<D>>()
            .unwrap()
            .subscribers
            .insert(reader);
        e.get_mut::<Observable<E>>()
            .unwrap()
            .subscribers
            .insert(reader);
        f.get_mut::<Observable<F>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
            &c.get::<Observable<C>>().unwrap().data,
            &d.get::<Observable<D>>().unwrap().data,
            &e.get::<Observable<E>>().unwrap().data,
            &f.get::<Observable<F>>().unwrap().data,
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
    > Derivable<T> for (Obs<A>, Obs<B>, Obs<C>, Obs<D>, Obs<E>, Obs<F>, Obs<G>)
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
            e.0.rctx_entity,
            e.1.rctx_entity,
            e.2.rctx_entity,
            e.3.rctx_entity,
            e.4.rctx_entity,
            e.5.rctx_entity,
            e.6.rctx_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g] =
            world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);
        c.get_mut::<Observable<C>>()
            .unwrap()
            .subscribers
            .insert(reader);
        d.get_mut::<Observable<D>>()
            .unwrap()
            .subscribers
            .insert(reader);
        e.get_mut::<Observable<E>>()
            .unwrap()
            .subscribers
            .insert(reader);
        f.get_mut::<Observable<F>>()
            .unwrap()
            .subscribers
            .insert(reader);
        g.get_mut::<Observable<G>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
            &c.get::<Observable<C>>().unwrap().data,
            &d.get::<Observable<D>>().unwrap().data,
            &e.get::<Observable<E>>().unwrap().data,
            &f.get::<Observable<F>>().unwrap().data,
            &g.get::<Observable<G>>().unwrap().data,
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
        Obs<A>,
        Obs<B>,
        Obs<C>,
        Obs<D>,
        Obs<E>,
        Obs<F>,
        Obs<G>,
        Obs<H>,
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
            e.0.rctx_entity,
            e.1.rctx_entity,
            e.2.rctx_entity,
            e.3.rctx_entity,
            e.4.rctx_entity,
            e.5.rctx_entity,
            e.6.rctx_entity,
            e.7.rctx_entity,
        ];
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] =
            world.get_many_entities_mut(entities).unwrap();

        a.get_mut::<Observable<A>>()
            .unwrap()
            .subscribers
            .insert(reader);
        b.get_mut::<Observable<B>>()
            .unwrap()
            .subscribers
            .insert(reader);
        c.get_mut::<Observable<C>>()
            .unwrap()
            .subscribers
            .insert(reader);
        d.get_mut::<Observable<D>>()
            .unwrap()
            .subscribers
            .insert(reader);
        e.get_mut::<Observable<E>>()
            .unwrap()
            .subscribers
            .insert(reader);
        f.get_mut::<Observable<F>>()
            .unwrap()
            .subscribers
            .insert(reader);
        g.get_mut::<Observable<G>>()
            .unwrap()
            .subscribers
            .insert(reader);
        h.get_mut::<Observable<H>>()
            .unwrap()
            .subscribers
            .insert(reader);

        derive_fn((
            &a.get::<Observable<A>>().unwrap().data,
            &b.get::<Observable<B>>().unwrap().data,
            &c.get::<Observable<C>>().unwrap().data,
            &d.get::<Observable<D>>().unwrap().data,
            &e.get::<Observable<E>>().unwrap().data,
            &f.get::<Observable<F>>().unwrap().data,
            &g.get::<Observable<G>>().unwrap().data,
            &h.get::<Observable<H>>().unwrap().data,
        ))
    }
}
