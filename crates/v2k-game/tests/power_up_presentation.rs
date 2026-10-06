use v2k_formats::models::AnimVars;
use v2k_game::session::GameSession;

fn system_session() -> GameSession {
    let dir = v2k_test_support::retail_dir();
    let mut session = GameSession::init(&dir).expect("retail fixture must load");
    session
        .load_auxiliary_ovl(3, 1)
        .expect("retail fixture must load");
    session
}

fn selected_power_up(session: &GameSession, selector: i32) -> (u16, Vec<u16>) {
    let wrapper = session.cache.global_model(82).expect("powerup wrapper");
    let mut vars = AnimVars::default();
    vars.dynamic[1] = selector;
    let wrapper_frame = wrapper.materialize(&vars);
    let instance = wrapper_frame
        .instances
        .first()
        .expect("authored powerup child");
    vars.registers = instance.registers;
    let mut billboard_ids = session
        .cache
        .global_model(usize::from(instance.model_id))
        .expect("selected powerup child")
        .materialize(&vars)
        .billboards
        .into_iter()
        .map(|billboard| billboard.id)
        .collect::<Vec<_>>();
    billboard_ids.sort_unstable();
    (instance.model_id, billboard_ids)
}

#[v2k_test_support::retail_test]
fn level_one_power_up_selectors_choose_their_authored_orb_colors() {
    let session = system_session();

    // The Level-1 weapon payload's selector 2 chooses powerup2: three cyan
    // orb billboards. Targetter's selector 0x3c wraps through the initializer
    // component word to 28 and chooses powerup28's blue-violet center plus
    // paired red and orange orbiters.
    assert_eq!(selected_power_up(&session, 2), (85, vec![613, 613, 613]));
    assert_eq!(
        selected_power_up(&session, 0x3c & 0x1f),
        (111, vec![614, 615, 615, 617, 617])
    );
}
