#include <runtime.h>
#include <exec.h>


static CONTINUATION_8_4(scan_listener,
                        execf, heap, operator, value *, perf,
                        value, value, value,
                        value, value, value, multiplicity);
static void scan_listener(execf n, heap h, operator op, value *r, perf p,
                          value er, value ar, value vr,
                          value e, value a, value v, multiplicity count)
{
    if (count > 0) {
        store(r, er, e);
        store(r, ar, a);
        store(r, vr, v);
        apply(n, h, p, op, r);
    }
}

#define sigbit(__sig, __p, __r) ((sig&(1<<__p))? register_ignore: __r)

static CONTINUATION_7_4(do_scan, block, perf, execf, int, value, value, value, heap, perf, operator, value *);
static void do_scan(block bk, perf p, execf n, int sig, value e, value a, value v,
                    heap h, perf pp, operator op, value *r)
{
    start_perf(p);
    if ((op == op_flush) || (op == op_close)) {
        apply(n, h, p, op, r);
        stop_perf(p, pp);
        return;
    }

    apply(bk->ev->reader, sig,
          cont(h, scan_listener, n, h, op, r, p,
               sigbit(sig, 2, e), sigbit(sig, 1, a), sigbit(sig, 0, v)),
          lookup(r, e), lookup(r, a), lookup(r, v));
    stop_perf(p, pp);
}

static inline boolean is_cap(unsigned char x) {return (x >= 'A') && (x <= 'Z');}

static execf build_scan(block bk, node n)
{
    vector ar = vector_get(n->arguments, 0);
    estring description = vector_get(ar, 0);
    int sig = 0;
    for (int i=0; i< 3; i++) {
        sig <<= 1;
        sig |= is_cap(description->body[i]);
    }
    return cont(bk->h, do_scan, bk,
                register_perf(bk->ev, n),
                resolve_cfg(bk, n, 0),
                sig,
                vector_get(ar, 1),
                vector_get(ar, 2),
                vector_get(ar, 3));

}

static CONTINUATION_8_4(do_insert, block, perf, execf, int, value, value, value, value, heap, perf, operator, value *) ;
static void do_insert(block bk, perf p, execf n, int deltam,
                      value uuid, value e, value a, value v,
                      heap h, perf pp, operator op, value *r)
{
    start_perf(p);
    if ((unsigned long)type_of(lookup(r, v)) == allocation_space) {
        prf("bad guy: %v\n", v);
    }
    
    if (op == op_insert) {
        apply(bk->ev->insert, uuid, lookup(r, e), lookup(r, a), lookup(r, v), deltam);
    }
    if (op == op_remove) {
        apply(bk->ev->insert, uuid, lookup(r, e), lookup(r, a), lookup(r, v), -deltam);
    }
    apply(n, h, p, op, r);
    stop_perf(p, pp);
}

static execf build_insert(block bk, node n)
{
    vector a = vector_get(n->arguments, 0);
    uuid x = table_find(bk->ev->scopes, vector_get(a, 0));
    return cont(bk->h, do_insert, bk, register_perf(bk->ev, n),
                resolve_cfg(bk, n, 0),
                1,
                x,
                vector_get(a, 1),
                vector_get(a, 2),
                vector_get(a, 3));
}

static execf build_remove(block bk, node n)
{
    vector a = vector_get(n->arguments, 0);
    uuid x = table_find(bk->ev->scopes, vector_get(a, 0));
    return cont(bk->h, do_insert,  bk, register_perf(bk->ev, n),
                resolve_cfg(bk, n, 0),
                -1,
                x,
                vector_get(a, 1),
                vector_get(a, 2),
                vector_get(a, 3));
}

static CONTINUATION_4_4(each_set_remove, block, value, value, uuid, value, value, value, multiplicity);
static void each_set_remove(block bk, uuid u, value e, value a, value etrash, value atrash, value v, multiplicity m)
{
    apply(bk->ev->insert, u, e, a, v, -1);
}

static CONTINUATION_7_4(do_set, block, perf, execf, value, value, value, value, heap, perf, operator, value *) ;
static void do_set(block bk, perf p, execf n, value u, value e, value a, value v,
                   heap h, perf pp, operator op, value *r)
{
    start_perf(p);
    u = lookup(r, u);
    value ev = lookup(r, e);
    value av=  lookup(r, a);
    apply(bk->ev->reader, s_EAv, cont(h, each_set_remove, bk, u, ev, av), ev, av, 0);
    apply(bk->ev->insert, u, ev, av, lookup(r, v), 1);
    apply(n, h, p, op, r);
    stop_perf(p, pp);
}

static execf build_set(block bk, node n)
{
    vector a = vector_get(n->arguments, 0);
    uuid x = table_find(bk->ev->scopes, vector_get(a, 0));
    return cont(bk->h, do_set,  bk, register_perf(bk->ev, n),
                resolve_cfg(bk, n, 0),
                x,
                vector_get(a, 1),
                vector_get(a, 2),
                vector_get(a, 3));
}

extern void register_edb_builders(table builders)
{
    table_set(builders, intern_cstring("insert"), build_insert);
    table_set(builders, intern_cstring("remove"), build_remove);
    table_set(builders, intern_cstring("set"), build_set);
    table_set(builders, intern_cstring("scan"), build_scan);
}
