use std::marker::PhantomData;

use bevy::{ecs::{component::Component, query::{With, WorldQuery}, system::EntityCommands}};

pub use bevy_statemachine_macros::exclusive_state;

pub trait ExclusiveState : Component {
    type WithoutState : WorldQuery;

    fn set_exclusive_state<'a, 'b, 'c, 'd>(self, commands: &'a mut EntityCommands<'b, 'c, 'd>) -> &'a mut EntityCommands<'b, 'c, 'd>;
}

pub struct WithState<T>(PhantomData<T>);

impl<T : ExclusiveState> WorldQuery for WithState<T> {
    type Fetch = <(With<T>, <T as ExclusiveState>::WithoutState) as WorldQuery>::Fetch;

    type State = <(With<T>, <T as ExclusiveState>::WithoutState) as WorldQuery>::State;
}

pub trait ExclusiveStateTransitionEx {
    fn transition<T: ExclusiveState>(&mut self, state: T) -> &mut Self;
}

impl ExclusiveStateTransitionEx for EntityCommands<'_, '_, '_> {
    fn transition<T: ExclusiveState>(&mut self, state: T) -> &mut Self {
        state.set_exclusive_state(self)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::{thread::sleep, time::Duration};
    use bevy::{prelude::*, utils::HashSet};

    exclusive_state!{
        #[derive(Debug)]
        enum MyState {
            CaseOne(pub(super) f32), // annoying to need pub here
            CaseTwo,
            CaseThree,
        }
    }
    
    struct Data {
        item: u32
    }

    fn state_one_system(
        mut commands: Commands,
        mut q: Query<(Entity, &mut Data, &MyState::CaseOne), WithState<MyState::CaseOne>>,
    ) {
        println!("begin exclusive_one");
        sleep(Duration::from_secs(1));

        for (e, mut data, case) in q.iter_mut() {
            println!("one({})->three {:?}", case.0, e);
            // switch state - expands to insert(state).remove::<CaseOne>().remove::<CaseTwo>()
            commands.entity(e).transition(MyState::CaseThree);
            data.item = 103;
        }

        println!("end exclusive_one");
    }

    fn state_two_system(
        mut commands: Commands,
        mut q: Query<(Entity, &mut Data), WithState<MyState::CaseTwo>>,
    ) {
        println!("begin exclusive_two");
        sleep(Duration::from_secs(1));

        for (e, mut data) in q.iter_mut() {
            println!("two->three {:?}", e);
            // switch state
            commands.entity(e).transition(MyState::CaseThree);
            data.item = 203;
        }

        println!("end exclusive_two");
    }

    #[test]
    pub fn test_exclusive() {
        let mut world = World::default();

        // optionally use sparse storage
        MyState::set_sparse(&mut world);

        // systems can run in parallel
        let mut stage = SystemStage::parallel();
        stage.add_system(state_one_system);
        stage.add_system(state_two_system);

        let one_id = world.spawn().insert(MyState::CaseOne(5.0)).insert(Data{ item: 1 }).id();
        let two_id = world.spawn().insert(MyState::CaseTwo).insert(Data{ item: 2 }).id();
        // bad setup - avoid this in practice by using EntityCommands::transition instead of raw insert
        let both_id = world.spawn().insert(MyState::CaseOne(6.0)).insert(Data{ item: 3 }).insert(MyState::CaseTwo).id(); 

        // exclusive state queries
        let q_x_one: HashSet<_> = world.query_filtered::<Entity, WithState<MyState::CaseOne>>().iter(&world).collect();
        assert!(q_x_one.contains(&one_id));
        assert!(q_x_one.len() == 1); // WithState does not include items with multiple states

        let q_x_two: HashSet<_> = world.query_filtered::<Entity, WithState<MyState::CaseTwo>>().iter(&world).collect();
        assert!(q_x_two.contains(&two_id));
        assert!(q_x_two.len() == 1);

        let q_x_three: HashSet<_> = world.query_filtered::<Entity, WithState<MyState::CaseThree>>().iter(&world).collect();
        assert!(q_x_three.is_empty());

        // access data 
        let q_x_one_data: Vec<_> = world.query_filtered::<&MyState::CaseOne, WithState<MyState::CaseOne>>().iter(&world).collect();
        assert!(q_x_one_data.len() == 1);
        assert!(q_x_one_data[0].0 == 5.0);

        stage.run(&mut world);

        // exclusive state queries
        let q_x_one: HashSet<_> = world.query_filtered::<Entity, WithState<MyState::CaseOne>>().iter(&world).collect();
        assert!(q_x_one.is_empty());
        
        let q_x_two: HashSet<_> = world.query_filtered::<Entity, WithState<MyState::CaseTwo>>().iter(&world).collect();
        assert!(q_x_two.is_empty());

        let q_x_three: HashSet<_> = world.query_filtered::<Entity, WithState<MyState::CaseThree>>().iter(&world).collect();
        assert!(q_x_three.contains(&one_id));
        assert!(q_x_three.contains(&two_id));
        assert!(q_x_three.len() == 2);

        // using as normal components will avoid exclusive filter
        let q_one: HashSet<_> = world.query_filtered::<Entity, With<MyState::CaseOne>>().iter(&world).collect();
        assert!(q_one.contains(&both_id));
        assert!(q_one.len() == 1);

        let q_two: HashSet<_> = world.query_filtered::<Entity, With<MyState::CaseTwo>>().iter(&world).collect();
        assert!(q_two.contains(&both_id));
        assert!(q_two.len() == 1);

        let q_three: HashSet<_> = world.query_filtered::<Entity, With<MyState::CaseThree>>().iter(&world).collect();
        assert!(q_three.contains(&one_id));
        assert!(q_three.contains(&two_id));
        assert!(q_three.len() == 2);

        // find items with any state (including invalid multiple states)
        let q_any: HashSet<_> = world.query_filtered::<Entity, WithMyState>().iter(&world).collect();
        assert!(q_any.len() == 3);
    }
}