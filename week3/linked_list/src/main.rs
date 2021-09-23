use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<String> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i.to_string());
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string()); // ToString impl for anything impl Display

    let mut clone_list = list.clone();
    println!("{}", list == clone_list);

    list.pop_front().unwrap();
    clone_list.push_front(String::from("32"));

    println!("{}", list == clone_list);

    for i in &list {
        print!("{} ", i);
    }
    println!("");

    println!("{}", list);

    // If you implement iterator trait:
    //for val in &list {
    //    println!("{}", val);
    //}
}
